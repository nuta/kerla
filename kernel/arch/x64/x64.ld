OUTPUT_FORMAT("elf64-x86-64");
ENTRY(boot);

LMA_OFFSET = 0xffff800000000000;

MEMORY {
    boot_ram (rwx): ORIGIN = 0x0000000000100000, LENGTH = 0x0001000
    ram (rwx):      ORIGIN = 0xffff800000101000, LENGTH = 0x2fff000
    lma_ram (rwx):  ORIGIN = 0x0000000000101000, LENGTH = 0x2fff000
}

SECTIONS {
    .boot : {
        __kernel_image = .; /* physical address */
        KEEP(*(.boot));
    } > boot_ram

    .text : ALIGN(8) {
        *(.text);
        *(.text.*);
    } > ram AT > lma_ram

    .rodata : ALIGN(8) {
        *(.rodata);
        *(.rodata.*);
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

        /* The kernel page table (physical addresses). */
        . = ALIGN(4096);
        __kernel_pml4 = . - LMA_OFFSET;
        . += 0x1000;
        __kernel_pdpt = . - LMA_OFFSET;
        . += 0x1000;
        __kernel_pd = . - LMA_OFFSET;
        . += 4 * 512 * 8; /* (# of PDPT entries) * (# of entries in PD) *
                             (size of entry) */

        /* The initial stack for BSP. We need reserve a large space since Rust
           tend to consume too much memory especially in the debug buid :/  */
        . += 0x10000;
        __boot_stack = . - LMA_OFFSET; /* physical address */

        . = ALIGN(4096);
        __kernel_image_end = . - LMA_OFFSET; /* physical address */
    } > ram AT > lma_ram

    /DISCARD/ :{
        *(*.eh_frame);
        *(*.eh_frame_hdr);
    }
}
