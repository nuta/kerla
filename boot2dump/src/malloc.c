#include <list.h>
#include <malloc.h>
#include <page_alloc.h>
#include <string.h>

#define NUM_BINS 16

static struct malloc_chunk *bins[NUM_BINS];

static void check_buffer_overflow(struct malloc_chunk *chunk) {
    if (chunk->magic == MALLOC_FREE) {
        return;
    }

    for (size_t i = 0; i < MALLOC_REDZONE_LEN; i++) {
        if (chunk->underflow_redzone[i] != MALLOC_REDZONE_UNDFLOW_MARKER) {
            PANIC("detected a malloc buffer underflow: ptr=%p", chunk->data);
        }
    }

    for (size_t i = 0; i < MALLOC_REDZONE_LEN; i++) {
        if (chunk->data[chunk->capacity + i] != MALLOC_REDZONE_OVRFLOW_MARKER) {
            PANIC("detected a malloc buffer overflow: ptr=%p", chunk->data);
        }
    }
}

static struct malloc_chunk *insert(void *ptr, size_t len) {
    ASSERT(len > MALLOC_FRAME_LEN);
    struct malloc_chunk *new_chunk = ptr;
    new_chunk->magic = MALLOC_FREE;
    new_chunk->capacity = len - MALLOC_FRAME_LEN;
    new_chunk->size = 0;
    new_chunk->next = NULL;

    // Append the new chunk into the linked list.
    struct malloc_chunk **chunk = &bins[NUM_BINS - 1];
    while (*chunk != NULL) {
        check_buffer_overflow(*chunk);
        chunk = &(*chunk)->next;
    }
    *chunk = new_chunk;

    return new_chunk;
}

static struct malloc_chunk *split(struct malloc_chunk *chunk, size_t len) {
    size_t new_chunk_len = MALLOC_FRAME_LEN + len;
    ASSERT(chunk->capacity >= new_chunk_len);

    void *new_chunk_ptr =
        &chunk->data[chunk->capacity + MALLOC_REDZONE_LEN - new_chunk_len];
    chunk->capacity -= new_chunk_len;

    ASSERT(new_chunk_len > MALLOC_FRAME_LEN);

    struct malloc_chunk *new_chunk = new_chunk_ptr;
    new_chunk->magic = MALLOC_FREE;
    new_chunk->capacity = len;
    new_chunk->size = 0;
    new_chunk->next = NULL;

    return new_chunk;
}

static int get_bin_idx_from_size(size_t size) {
    // If requested size is less or equal to the size of second largest chunk
    // (the last fixed chunk).
    for (size_t i = 0; i < NUM_BINS - 1; i++) {
        if (size <= 1 << i) {
            return i;
        }
    }

    // Return -1 indicating the last, dynamic-sized chunk
    return -1;
}

void *malloc(size_t size) {
    if (!size) {
        size = 1;
    }

    // Align up to 16-bytes boundary. If the size is less than 16 (including
    // size == 0), allocate 16 bytes.
    size = ALIGN_UP(size, 16);

    int bin_idx = get_bin_idx_from_size(size);

    if (bin_idx != -1 && bins[bin_idx] != NULL) {
        // Check the list corresponding to that size for a free chunk.
        struct malloc_chunk *allocated = bins[bin_idx];
        ASSERT(allocated->magic == MALLOC_FREE);

        allocated->magic = MALLOC_IN_USE;
        allocated->size = size;
        memset(allocated->underflow_redzone, MALLOC_REDZONE_UNDFLOW_MARKER,
               MALLOC_REDZONE_LEN);
        memset(&allocated->data[allocated->capacity],
               MALLOC_REDZONE_OVRFLOW_MARKER, MALLOC_REDZONE_LEN);

        bins[bin_idx] = allocated->next;
        allocated->next = NULL;
        return allocated->data;
    }

    struct malloc_chunk *prev = NULL;
    for (struct malloc_chunk *chunk = bins[NUM_BINS - 1]; chunk;
         chunk = chunk->next) {
        ASSERT(chunk->magic == MALLOC_FREE);

        struct malloc_chunk *allocated = NULL;
        size_t chunk_size = bin_idx < 0 ? size : (1 << bin_idx);
        if (chunk->capacity > chunk_size + MALLOC_FRAME_LEN) {
            allocated = split(chunk, chunk_size);
        } else if (chunk->capacity >= chunk_size) {
            allocated = chunk;
            // Remove chunk from the linked list.
            if (prev) {
                // If it was not at the head of the list.
                prev->next = chunk->next;
            } else {
                // If it was at the head of the list.
                bins[NUM_BINS - 1] = bins[NUM_BINS - 1]->next;
            }
        }

        if (allocated) {
            allocated->magic = MALLOC_IN_USE;
            allocated->size = size;
            memset(allocated->underflow_redzone, MALLOC_REDZONE_UNDFLOW_MARKER,
                   MALLOC_REDZONE_LEN);
            memset(&allocated->data[allocated->capacity],
                   MALLOC_REDZONE_OVRFLOW_MARKER, MALLOC_REDZONE_LEN);
            allocated->next = NULL;
            return allocated->data;
        }
        prev = chunk;
    }

    PANIC("out of memory");
}

static struct malloc_chunk *get_chunk_from_ptr(void *ptr) {
    struct malloc_chunk *chunk =
        (struct malloc_chunk *) ((uintptr_t) ptr - sizeof(struct malloc_chunk));

    // Check its magic and underflow/overflow redzones.
    ASSERT(chunk->magic == MALLOC_IN_USE);
    check_buffer_overflow(chunk);
    return chunk;
}

void free(void *ptr) {
    if (!ptr) {
        return;
    }
    struct malloc_chunk *chunk = get_chunk_from_ptr(ptr);
    if (chunk->magic == MALLOC_FREE) {
        PANIC("double-free bug!");
    }

    chunk->magic = MALLOC_FREE;

    int bin_idx = get_bin_idx_from_size(chunk->capacity);
    bin_idx = bin_idx < 0 ? NUM_BINS - 1 : bin_idx;

    struct malloc_chunk *head = bins[bin_idx];
    if (head) {
        chunk->next = head;
    }
    bins[bin_idx] = chunk;
}

void *realloc(void *ptr, size_t size) {
    if (!ptr) {
        return malloc(size);
    }

    struct malloc_chunk *chunk = get_chunk_from_ptr(ptr);
    size_t prev_size = chunk->size;
    if (size <= chunk->capacity) {
        // There's enough room. Keep using the current chunk.
        return ptr;
    }

    // There's not enough room. Allocate a new space and copy old data.
    void *new_ptr = malloc(size);
    memcpy(new_ptr, ptr, prev_size);
    free(ptr);
    return new_ptr;
}

void malloc_init(void) {
    STATIC_ASSERT(IS_ALIGNED(HEAP_SIZE, PAGE_SIZE));
    insert(page_alloc(HEAP_SIZE / PAGE_SIZE, false), HEAP_SIZE);
}
