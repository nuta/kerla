from . import Package

MAIN_C = """\
#include <stdio.h>

int main(void) {
    puts("Hello World!");
    return 0;
}
"""


class HelloWorld(Package):
    def __init__(self):
        super().__init__()
        self.name = "hello_world"
        self.version = "0.0.1"
        self.host_deps = ["gcc"]
        self.files = {
            "/bin/hello": "hello",
        }

    def build(self):
        self.add_file("main.c", MAIN_C)
        self.run(["gcc", "-O0", "-g3", "-static", "-o", "hello", "main.c"])
