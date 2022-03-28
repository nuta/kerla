#ifndef __PRINTF_H__
#define __PRINTF_H__

#include <types.h>

void printf(const char *fmt, ...);

// ANSI escape sequences (SGR).
#define SGR_ERR      "\e[1;91m"  // Bold red.
#define SGR_WARN     "\e[0;33m"  // Yellow.
#define SGR_WARN_DBG "\e[1;33m"  // Bold yellow.
#define SGR_DEBUG    "\e[1;32m"  // Bold green.
#define SGR_RESET    "\e[0m"

#ifdef RELEASE
#    define TRACE(...)
#else
#    define TRACE(fmt, ...)                                                    \
        do {                                                                   \
            printf("[boot2dump] " fmt "\n", ##__VA_ARGS__);                    \
        } while (0)
#endif

#define DBG(fmt, ...)                                                          \
    do {                                                                       \
        printf(SGR_DEBUG "[boot2dump] " fmt SGR_RESET "\n", ##__VA_ARGS__);    \
    } while (0)

#define INFO(fmt, ...)                                                         \
    do {                                                                       \
        printf("[boot2dump] " fmt "\n", ##__VA_ARGS__);                        \
    } while (0)

#define WARN(fmt, ...)                                                         \
    do {                                                                       \
        printf(SGR_WARN "[boot2dump] WARN: " fmt SGR_RESET "\n",               \
               ##__VA_ARGS__);                                                 \
    } while (0)

#define WARN_DBG(fmt, ...)                                                     \
    do {                                                                       \
        printf(SGR_WARN_DBG "[boot2dump] WARN: " fmt SGR_RESET "\n",           \
               ##__VA_ARGS__);                                                 \
    } while (0)

#define HEXDUMP(ptr, len)                                                      \
    do {                                                                       \
        uint8_t *__ptr = (uint8_t *) (ptr);                                    \
        size_t __len = len;                                                    \
        printf("%04x: ", 0);                                                   \
        size_t i;                                                              \
        for (i = 0; i < __len; i++) {                                          \
            if (i > 0 && i % 16 == 0) {                                        \
                printf("\n");                                                  \
                printf("%04x: ", i);                                           \
            }                                                                  \
                                                                               \
            printf("%02x ", __ptr[i]);                                         \
        }                                                                      \
                                                                               \
        printf("\n");                                                          \
    } while (0)

#define ASSERT(expr)                                                           \
    do {                                                                       \
        if (!(expr)) {                                                         \
            printf(SGR_ERR                                                     \
                   "[boot2dump] %s:%d: ASSERTION FAILURE: %s" SGR_RESET "\n",  \
                   __FILE__, __LINE__, #expr);                                 \
            arch_halt();                                                       \
            __builtin_unreachable();                                           \
        }                                                                      \
    } while (0)

#define DEBUG_ASSERT(expr) ASSERT(expr)

#define PANIC(fmt, ...)                                                        \
    do {                                                                       \
        printf(SGR_ERR "[boot2dump] PANIC: " fmt SGR_RESET "\n",               \
               ##__VA_ARGS__);                                                 \
        arch_halt();                                                           \
        __builtin_unreachable();                                               \
    } while (0)

#define NYI()                                                                  \
    do {                                                                       \
        printf(SGR_ERR "[boot2dump] %s(): not yet ymplemented: %s:%d\n",       \
               __func__, __FILE__, __LINE__);                                  \
        arch_halt();                                                           \
        __builtin_unreachable();                                               \
    } while (0)

#define UNREACHABLE()                                                          \
    do {                                                                       \
        printf(SGR_ERR "[boot2dump] Unreachable at %s:%d (%s)\n" SGR_RESET,    \
               __FILE__, __LINE__, __func__);                                  \
        arch_halt();                                                           \
        __builtin_unreachable();                                               \
    } while (0)

#endif
