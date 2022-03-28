OUTPUT_FORMAT("elf64-x86-64");
ENTRY(boot);

LMA_OFFSET = 0xffff800000000000;

MEMORY {
    boot_ram (rwx): ORIGIN = 0x0000000000100000, LENGTH = 0x0001000
    ram (rwx):      ORIGIN = 0xffff800000101000, LENGTH = 0x3eff000
    lma_ram (rwx):  ORIGIN = 0x0000000000101000, LENGTH = 0x3eff000
}

SECTIONS {
    . = 0x5000;
    __mp_boot_trampoine = .;
    . = 0x6000;
    __mp_boot_trampoine_end = .;
    __mp_boot_gdtr = .;

    .boot : {
        __kernel_image = .; /* paddr_t */
        KEEP(*(.boot));
    } > boot_ram

    .text : ALIGN(8) {
        *(.text);
        *(.text.*);
    } > ram AT > lma_ram

    .data : ALIGN(8) {
        *(.data);
        *(.data.*);
    } > ram AT > lma_ram

    .bss : ALIGN(8) {
        __bss = .;
        *(.bss);
        *(.bss.*);
        __bss_end = .;

        /* The kernel page table (paddr_t). */
        . = ALIGN(4096);
        __kernel_pml4 = . - LMA_OFFSET;
        . += 0x1000;
        __kernel_pdpt = . - LMA_OFFSET;
        . += 0x1000;
        __kernel_pd = . - LMA_OFFSET;
        . += 4 * 512 * 8; /* (# of PDPT entries) * (# of entries in PD) *
                             (size of entry) */

        /* CPU variables and boot stacks (paddr_t). */
        . = ALIGN(4096);
        __boot_stack_base = . - LMA_OFFSET;
        . += 0x1000 * 16; /* PAGE_SIZE * CPU_NUM_MAX */

        . = ALIGN(4096);
        __kernel_image_end = . - LMA_OFFSET; /* paddr_t */
    } > ram AT > lma_ram

    .rodata : ALIGN(8) {
        *(.rodata);
        *(.rodata.*);
    } > ram AT > lma_ram

    /DISCARD/ :{
        *(*.eh_frame);
    }
}
