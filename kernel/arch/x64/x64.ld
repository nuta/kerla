OUTPUT_FORMAT("elf64-x86-64");
ENTRY(boot);

VMA_OFFSET  = 0xffff800000000000;
KERNEL_BASE = VMA_OFFSET + 0x100000;

SECTIONS {
    . = KERNEL_BASE - VMA_OFFSET;

    .boot : {
        __kernel_image = .; /* physical address */
        KEEP(*(.boot));
    }

    . = KERNEL_BASE + SIZEOF(.boot);

    .text : AT(ADDR(.text) - VMA_OFFSET) {
        *(.text);
        *(.text.*);
    }

    .rodata : AT(ADDR(.rodata) - VMA_OFFSET) {
        *(.rodata);
        *(.rodata.*);

        . = ALIGN(4096);
        __cpu_local = .;
        KEEP(*(.cpu_local_head));
        KEEP(*(.cpu_local));
        __cpu_local_end = .;
        __cpu_local_size = __cpu_local_end - __cpu_local;
    }

    .data : AT(ADDR(.data) - VMA_OFFSET) {
        *(.data);
        *(.data.*);

        *(.got*);

        __bsp_cpu_local = .;
        . += __cpu_local_size;
        __bsp_cpu_local_end = .;
    }

    .bss : AT(ADDR(.bss) - VMA_OFFSET) {
        __bss = .;
        *(.bss);
        *(.bss.*);
        __bss_end = .;

        /* The kernel page table (physical addresses). */
        . = ALIGN(4096);
        __kernel_pml4 = . - VMA_OFFSET;
        . += 0x1000;
        __kernel_pdpt = . - VMA_OFFSET;
        . += 0x1000;
        __kernel_pd = . - VMA_OFFSET;
        . += 4 * 512 * 8; /* (# of PDPT entries) * (# of entries in PD) *
                             (size of entry) */

        /* The initial stack for BSP. We need reserve a large space since Rust
           tend to consume too much memory especially in the debug buid :/  */
        . += 0x10000;
        __boot_stack = . - VMA_OFFSET; /* physical address */

        . = ALIGN(4096);
        __kernel_image_end = . - VMA_OFFSET; /* physical address */
    }

    /DISCARD/ :{
        *(*.eh_frame);
        *(*.eh_frame_hdr);
    }
}
