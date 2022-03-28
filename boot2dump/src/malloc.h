#ifndef __MALLOC_H__
#define __MALLOC_H__

#include <types.h>

#define HEAP_SIZE (64 * 1024)

#define MALLOC_FREE        0x0a110ced0a110cedULL /* hexspeak of "alloced" */
#define MALLOC_IN_USE      0xdea110cddea110cdULL /* hexspeak of "deallocd" */
#define MALLOC_REDZONE_LEN 16
#define MALLOC_FRAME_LEN   (sizeof(struct malloc_chunk) + MALLOC_REDZONE_LEN)

#define MALLOC_REDZONE_UNDFLOW_MARKER 0x5a
#define MALLOC_REDZONE_OVRFLOW_MARKER 0x5b

/// The header of allocated/free memory chunks. The data area follows
//  immediately after this header (`data` points to the area).
struct malloc_chunk {
    struct malloc_chunk *next;
    size_t capacity;
    size_t size;
    uint64_t magic;
    uint8_t underflow_redzone[MALLOC_REDZONE_LEN];
    uint8_t data[];
    // `overflow_redzone` follows immediately after `data`.
    // uint8_t overflow_redzone[MALLOC_REDZONE_LEN];
};

/// Ensure that it's aligned to 16 bytes for performance (SSE instructions
/// requires 128-bit-aligned memory address).
STATIC_ASSERT(sizeof(struct malloc_chunk) == 48);

void *malloc(size_t size);
void *realloc(void *ptr, size_t size);
void free(void *ptr);
void malloc_init(void);

#endif
