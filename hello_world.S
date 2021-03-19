.code64
.intel_syntax noprefix
.global _start
_start:
    /* write(stdout, "Hello, World from the userspace!\n", 32) */
    mov rax, 1
    mov rdi, 1
    lea rsi, [rip + msg]
    mov rdx, 32
    syscall

    /* exit(0) */
    mov rax, 60
    mov rdi, 0
    syscall

    ud2

.section .rodata
msg:
    .ascii "Hello World from the userspace!\n"
