.section ".boot", "ax"
.intel_syntax noprefix

#define MULTIBOOT_MAGIC    0x1badb002
#define MULTIBOOT_FLAGS    0x00000040 // memory map
#define MULTIBOOT_CHECKSUM -(MULTIBOOT_MAGIC + MULTIBOOT_FLAGS)

.code32

// The multiboot header.
.align 8
.long MULTIBOOT_MAGIC
.long MULTIBOOT_FLAGS
.long MULTIBOOT_CHECKSUM

// The entry point jumped from the bootloader.
.code32
.global boot
boot:
    cli
    cld

    // Set the boot (later reused for the cpu-local idle thread) stack.
    lea esp, [__boot_stack_base + 0x1000]

    // Save the 64-bit physical address of struct multiboot_info.
    xor eax, eax
    push eax // Upper 32-bits.
    push ebx

    // Prepare for RETF.
    mov eax, 24
    push eax
    lea edx, [protected_mode]
    push edx

    // Switch to our own temporary GDT.
    lgdt [boot_gdtr]
    retf

protected_mode:
    mov ax, 16
    mov ds, ax
    mov ss, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

// Each PDPT table maps 1GiB space. We map 4GiB for kernel to access
// the memory-mapped Local APIC registers.
#define NUM_PDPT_ENTRIES 4

construct_page_table:
    // PML4: 0x00000000_00000000 (temporarily used in protected mode)
    lea edi, [__kernel_pml4]
    lea eax, [__kernel_pdpt + 0x103] // Present, writable, global.
    mov dword ptr [edi], eax
    mov dword ptr [edi + 4], 0

    // PML4: 0xffff8000_00000000
    lea edi, [__kernel_pml4 + 256 * 8]
    lea eax, [__kernel_pdpt + 0x103] // Present, writable, global.
    mov dword ptr [edi], eax
    mov dword ptr [edi + 4], 0

    // PDPT
    lea edi, [__kernel_pdpt]
    lea eax, [__kernel_pd + 0x103] // Present, writable, global.
    mov ecx, NUM_PDPT_ENTRIES

write_pdpt_entry:
    mov dword ptr [edi], eax
    mov dword ptr [edi + 4], 0
    add eax, 0x1000
    add edi, 8
    loop write_pdpt_entry

    // Page Directory
    lea edi, [__kernel_pd]
    mov eax, 0x0000183 // Present, writable, global, page size is 2MB.
    mov ecx, NUM_PDPT_ENTRIES * 512 // (# of PDPT entries) * (# of entries in PD)

write_pd_entry:
    mov dword ptr [edi], eax
    mov dword ptr [edi + 4], 0
    add eax, 0x200000 // 2MB
    add edi, 8
    loop write_pd_entry

    jmp enable_long_mode

//
//  Common boot code for both BSP and APs.
//
enable_long_mode:
    // Enable PAE and PGE.
    mov eax, cr4
    or  eax, 0xa0
    mov cr4, eax

    // Set the page table address.
    lea eax, [__kernel_pml4]
    mov cr3, eax

    // Enable long mode.
    mov ecx, 0xc0000080
    rdmsr
    or  eax, 0x0100
    wrmsr

    // Prepare for RETF.
    mov  eax, 8
    push eax
    lea  edx, [long_mode_in_low_address]
    push edx

    // Enable paging.
    mov eax, cr0
    or  eax, 0x80000000
    mov cr0, eax

    retf

// Temporary GDTR/GDT entries. This must be located in the .boot section as its
// address (gdt) must be physical to load.
.align 16
.global boot_gdtr
boot_gdtr:
    .word gdt_end - gdt - 1
    .quad gdt

.align 16
gdt:
    .quad 0x0000000000000000 // 0:  null descriptor
    .quad 0x00af9a000000ffff // 8:  64-bit code segment (kernel)
    .quad 0x00cf92000000ffff // 16: 64-bit data segment (kernel)
    .quad 0x00cf9a000000ffff // 24: 32-bit code segment (kernel)
gdt_end:

.code64
long_mode_in_low_address:
    mov ax, 0
    mov ds, ax
    mov ss, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    // Update RSP/RIP to use the virtual address.
    mov rbx, 0xffff800000000000
    or  rsp, rbx
    lea rax, [long_mode - 0xffff800000000000]
    or  rax, rbx
    jmp rax

//
//  From here, we're in the .text section: we no longer use physical address.
//
.code64
.text
long_mode:
    // Clear .bss section
    mov al, 0x00
    lea rdi, [rip + __bss]
    lea rcx, [rip + __bss_end]
    sub rcx, rdi
    cld
    rep stosb

    pop  rdi // Restore the address of multiboot_info
    lea  rax, [rip + test_main]
    call rax

// Unreachable here.
halt:
    cli
    hlt
    jmp halt

.rodata
.align 4096
.global __boot2dump
__boot2dump:
    .incbin "build/boot2dump.bin"
