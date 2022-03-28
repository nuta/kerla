#ifndef __TYPES_H__
#define __TYPES_H__

typedef char int8_t;
typedef short int16_t;
typedef int int32_t;
typedef long long int64_t;
typedef unsigned char uint8_t;
typedef unsigned short uint16_t;
typedef unsigned uint32_t;
typedef unsigned long long uint64_t;

#ifdef __LP64__
typedef int64_t intmax_t;
typedef uint64_t uintmax_t;
#else
typedef int32_t intmax_t;
typedef uint32_t uintmax_t;
#endif

typedef uintmax_t size_t;
typedef uintmax_t paddr_t;
typedef uintmax_t vaddr_t;
typedef uintmax_t uintptr_t;
typedef intmax_t ptrdiff_t;
typedef uintmax_t offset_t;

#define INT8_MIN   -128
#define INT16_MIN  -32768
#define INT32_MIN  -2147483648
#define INT64_MIN  -9223372036854775808LL
#define INT8_MAX   127
#define INT16_MAX  32767
#define INT32_MAX  2147483647
#define INT64_MAX  9223372036854775807LL
#define UINT8_MAX  255
#define UINT16_MAX 65535
#define UINT32_MAX 4294967295U
#define UINT64_MAX 18446744073709551615ULL

typedef char bool;
#define true 1
#define false 0
#define NULL ((void *) 0)

typedef __builtin_va_list va_list;
#define offsetof(type, field) __builtin_offsetof(type, field)
#define is_constant(expr)     __builtin_constant_p(expr)
#define va_start(ap, param)   __builtin_va_start(ap, param)
#define va_end(ap)            __builtin_va_end(ap)
#define va_arg(ap, type)      __builtin_va_arg(ap, type)
#define __unused              __attribute__((unused))
#define __packed              __attribute__((packed))
#define __noreturn            __attribute__((noreturn))
#define __weak                __attribute__((weak))
#define __mustuse             __attribute__((warn_unused_result))
/// Indicates the value is never NULL.
#define __nonnull
/// Indicates the argument or return value is nullable.
#define __nullable
/// Indicates the return value can be `error_t`. Use `IS_OK`.
#define __failable
#define MEMORY_BARRIER()         __asm__ __volatile__("" ::: "memory");
#define __aligned(aligned_to)    __attribute__((aligned(aligned_to)))
#define ALIGN_DOWN(value, align) ((value) & ~((align) -1))
#define ALIGN_UP(value, align)   ALIGN_DOWN((value) + (align) -1, align)
#define IS_ALIGNED(value, align) (((value) & ((align) -1)) == 0)
#define STATIC_ASSERT(expr)      _Static_assert(expr, #expr);
#define MAX(a, b)                                                              \
    ({                                                                         \
        __typeof__(a) __a = (a);                                               \
        __typeof__(b) __b = (b);                                               \
        (__a > __b) ? __a : __b;                                               \
    })
#define MIN(a, b)                                                              \
    ({                                                                         \
        __typeof__(a) __a = (a);                                               \
        __typeof__(b) __b = (b);                                               \
        (__a < __b) ? __a : __b;                                               \
    })

/// Error codes. All errors are negative and positive values are used for
/// representing a success in return values.
typedef int error_t;
#define OK            0
#define ERR_NO_MEMORY -1
#define ERR_EMPTY     -2
#define ERR_NOT_FOUND -3

#define SECTOR_SIZE 512

#include <arch_types.h>

#endif
