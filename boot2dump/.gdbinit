file build/boot2dump.elf
target remote 127.0.0.1:1234

set confirm off
set history save on
set print pretty on
set disassemble-next-line auto

set riscv use-compressed-breakpoints yes

