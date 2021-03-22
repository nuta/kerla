.intel_syntax noprefix

/// void copy_from_user(void *dst, const void *src, size_t len);
/// void copy_to_user(void *dst, const void *src, size_t len);
///
/// Copies a memory buffer from/to the user space. We don't check the validity
/// of the user's addresses; instead, we handles a page fault occurred at
/// `usercopy` as a user's fault.
///
/// Note that caller MUST check that the memory range does not overlaps with the
/// kernel sapce!
.global copy_from_user, copy_to_user, usercopy
copy_from_user:
copy_to_user:
    mov rcx, rdx
    cld
usercopy:
    rep movsb
    ret
