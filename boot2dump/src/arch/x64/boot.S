.intel_syntax noprefix
.section .boot, "ax"

// The entry point of boot2dump. Here we assume that:
//
//   - The CPU is in the long mode.
//   - Memory pages where the boot2dump image exists are straight mapped into
//     and the straight mapping starts from `0xffff_8000_0000_0000`
//     (i.e. `0xffff_8000_0001_0000` points to `0x1_0000`).
.global __base, boot
__base:
boot:
    cli

    // Initialize segment registers just in case.
    xor ax, ax
    mov ds, ax
    mov ss, ax
    mov gs, ax
    mov fs, ax

    // Initialize stack pointer and frame pointer.
    lea rsp, [rip + __stack_end]
    mov rbp, 0

    // Save arguments for main.
    push rdi
    push rsi
    push rdx
    push rcx


    // Start the program.
    call x64_init

    // Restore arguments for main and call it.
    pop rcx
    pop rdx
    pop rsi
    pop rdi
    call main

    // Here's unreachable, I hope.
    cli
    hlt
