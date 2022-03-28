OUTPUT_FORMAT("elf64-x86-64");
ENTRY(boot);

SECTIONS {
    . = 0;

    .text : {
        KEEP(*(.boot));
        *(.text);
        *(.text.*);
    }

    .rela.dyn : ALIGN(16) {
        __rela_dyn = .;
        *(.rela.dyn);
        *(.rela.dyn.*);
        __rela_dyn_end = .;
    }

    .rodata : ALIGN(16) {
        *(.rodata);
        *(.rodata.*);
    }

    .data : ALIGN(16) {
        *(.data);
        *(.data.*);

        __bss = .;
        *(.bss);
        *(.bss.*);
        __bss_end = .;

        __stack = .;
        . += 128 * 1024;
        . = ALIGN(4096);
        __stack_end = .;

        __free_pages = .;
        . += 512 * 1024;
        __free_pages_end = .;
    }

    .bss :{
        /*
         * .bss section is deliberately moved to .data so that objcopy outputs
         * that zero-filled memory space into the raw binary file, in other words,
         * reserve .bss section for boot2dump in the binary file.
         */
    }
}
