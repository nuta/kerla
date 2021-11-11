.section ".boot", "ax"

.code32

// Multiboot2 header
// https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html
.align 8
multiboot2_header:
  .long 0xe85250d6                                      // magic
  .long 0                                               // arch = i386
  .long multiboot2_header_end - multiboot2_header       // length
  .long -(0xe85250d6 + 0 + (multiboot2_header_end - multiboot2_header)) // chechsum

// tag: information request
info_request:
  .short 1
  .short 0
  .long  info_request_end - info_request
  .long  6 // memory map
info_request_end:

.align 8
 // tag: terminator
  .short 0
  .short 0
  .long  8
multiboot2_header_end:

// The (old) multiboot header for QEMU -kernel.
// https://www.gnu.org/software/grub/manual/multiboot/multiboot.html
.align 8
multiboot_header:
.long 0x1badb002
.long 0x00000000
.long -(0x1badb002 + 0x00000000)

// The entry point jumped from the bootloader.
.code64
.global boot
boot:
    // Check if the CPU is in the 64-bit mode: this block must be valid for
    // both 64-bit and 32-bit modes.
    //
    // We support the following boot protocols:
    //
    //   GRUB: Multiboot 0.x or 2 (32-bit), EBX points to the multiboot info
    //   Firecracker: Linux/x86 Boot Protocol (64-bit), ESI points to boot_params
    mov edi, eax
    mov ecx, 0xc0000080
    rdmsr
    test  eax, 0x0100
    mov eax, edi
    jz boot32

    // Booted directly in 64-bit mode. To simplify the boot code, we switch back
    // into the 32-bit mode and start executing the 32-bit boot code.
    lgdt [boot_gdtr]
    mov eax, 0xb002b002 // magic for boot_params
    mov ebx, esi        // struct boot_params *
    sub rsp, 8
    mov dword ptr [rsp], offset swtich_back_to_32bit
    mov dword ptr [rsp + 4], 24
    retf

.code32
swtich_back_to_32bit:
    mov dx, 16
    mov ds, dx
    mov ss, dx
    mov es, dx
    mov fs, dx
    mov gs, dx

.code32
boot32:
    cli
    cld

    // Set the boot (later reused for the cpu-local idle thread) stack.
    mov esp, offset __boot_stack

    // Save the multiboot magic and 64-bit physical address of multiboot info onto the stack.
    push 0      // Upper 32-bits.
    push eax
    push 0      // Upper 32-bits.
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
    mov ecx, 4 // (# of PDPT entries)

write_pdpt_entry:
    mov dword ptr [edi], eax
    mov dword ptr [edi + 4], 0
    add eax, 0x1000
    add edi, 8
    loop write_pdpt_entry

    // Page Directory
    lea edi, [__kernel_pd]
    mov eax, 0x0000183 // Present, writable, global, page size is 2MB.
    mov ecx, 4 * 512 // (# of PDPT entries) * (# of entries in PD)

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
    // Determine whether the current CPU is BSP or AP.
    mov esi, 0xfee00020
    mov eax, [esi]
    shr eax, 24
    test eax, eax
    jz  setup_bsp

setup_ap:
    // TODO: SMP
    ud2

setup_bsp:
    // Clear .bss section
    mov al, 0x00
    lea rdi, [rip + __bss]
    lea rcx, [rip + __bss_end]
    sub rcx, rdi
    cld
    rep stosb

    pop  rsi // the address of multiboot info
    pop  rdi // multiboot magic

    // Clear the frame pointer to stop backtracing here.
    xor rbp, rbp

    lea  rax, [rip + bsp_early_init]
    call rax

    // In case init() returns.
halt:
    cli
    hlt
    jmp halt
