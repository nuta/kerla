//
// Undefined Behavior Sanitizer runtime (UBSan).
// https://clang.llvm.org/docs/UndefinedBehaviorSanitizer.html
//
#include "ubsan.h"
#include "printf.h"

static void report_ubsan_event(const char *event) {
    PANIC("detected an undefined behavior: %s", event);
}

void __ubsan_handle_type_mismatch_v1(struct ubsan_mismatch_data_v1 *data,
                                     vaddr_t ptr) {
    if (!ptr) {
        report_ubsan_event("NULL pointer dereference");
    } else if (data->align != 0 && (ptr & ((1 << data->align) - 1)) != 0) {
        PANIC("pointer %p is not aligned to %d", ptr, 1 << data->align);
    } else {
        PANIC("pointer %p is not large enough for %s", ptr, data->type->name);
    }
}

void __ubsan_handle_add_overflow(void) {
    report_ubsan_event("add_overflow");
}
void __ubsan_handle_sub_overflow(void) {
    report_ubsan_event("sub overflow");
}
void __ubsan_handle_mul_overflow(void) {
    report_ubsan_event("mul overflow");
}
void __ubsan_handle_divrem_overflow(void) {
    report_ubsan_event("divrem overflow");
}
void __ubsan_handle_negate_overflow(void) {
    report_ubsan_event("negate overflow");
}
void __ubsan_handle_float_cast_overflow(void) {
    report_ubsan_event("float cast overflow");
}
void __ubsan_handle_pointer_overflow(void) {
    report_ubsan_event("pointer overflow");
}
void __ubsan_handle_out_of_bounds(void) {
    report_ubsan_event("out of bounds");
}
void __ubsan_handle_shift_out_of_bounds(void) {
    report_ubsan_event("shift out of bounds");
}
void __ubsan_handle_builtin_unreachable(void) {
    report_ubsan_event("builtin unreachable");
}
void __ubsan_handle_invalid_builtin(void) {
    report_ubsan_event("invalid builtin");
}
