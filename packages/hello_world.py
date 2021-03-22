from . import Package

SOURCE = """
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
"""


class HelloWorld(Package):
    def __init__(self):
        super().__init__()
        self.name = "hello_world"
        self.version = "0.0.1"
        self.host_deps = ["gcc"]
        self.files = {
            "/sbin/init": "hello_world"
        }
        self.symlinks = {}

    def build(self):
        self.add_file("hello_world.S", SOURCE)
        self.run(
            "as hello_world.S -o hello_world.o && ld hello_world.o -o hello_world")
