#include "page_alloc.h"
#include <printf.h>
#include <string.h>

extern char __free_pages[];
extern char __free_pages_end[];

vaddr_t current = (vaddr_t) __free_pages;

/// Allocates `n` pages of memory.
void *page_alloc(int n, bool zeroed) {
    DEBUG_ASSERT(IS_ALIGNED(current, PAGE_SIZE));

    if (n <= 0) {
        PANIC("page_alloc: num_pages <= 0");
    }

    if (current + n * PAGE_SIZE > (vaddr_t) __free_pages_end) {
        PANIC("page_alloc: out of memory");
    }

    void *ptr = (void *) current;
    current += n * PAGE_SIZE;

    if (zeroed) {
        memset(ptr, 0, n * PAGE_SIZE);
    }

    return ptr;
}
