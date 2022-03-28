.intel_syntax noprefix
.text

.global memcpy
memcpy:
    mov rcx, rdx
    cld
    rep movsb
    ret

.global memset
memset:
    mov al, sil
    mov rcx, rdx
    cld
    rep stosb
    ret
