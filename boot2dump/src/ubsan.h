#ifndef __UBSAN_H__
#define __UBSAN_H__

#include <types.h>

struct ubsan_type {
    uint16_t kind;
    uint16_t info;
    char name[];
};

struct ubsan_sourceloc {
    const char *file;
    uint32_t line;
    uint32_t column;
};

struct ubsan_mismatch_data_v1 {
    struct ubsan_sourceloc loc;
    struct ubsan_type *type;
    uint8_t align;
    uint8_t kind;
};

void __ubsan_handle_type_mismatch_v1(struct ubsan_mismatch_data_v1 *data,
                                     vaddr_t ptr);
void __ubsan_handle_add_overflow(void);
void __ubsan_handle_sub_overflow(void);
void __ubsan_handle_mul_overflow(void);
void __ubsan_handle_divrem_overflow(void);
void __ubsan_handle_negate_overflow(void);
void __ubsan_handle_float_cast_overflow(void);
void __ubsan_handle_pointer_overflow(void);
void __ubsan_handle_out_of_bounds(void);
void __ubsan_handle_shift_out_of_bounds(void);
void __ubsan_handle_builtin_unreachable(void);
void __ubsan_handle_invalid_builtin(void);

#endif
