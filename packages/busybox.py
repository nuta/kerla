from . import Package

COMMANDS = [
    "sh", "echo", "cat", "uname", "nslookup",
]


class Busybox(Package):
    def __init__(self):
        super().__init__()
        self.name = "busybox"
        self.version = "1.31.1"
        self.url = f"https://busybox.net/downloads/busybox-{self.version}.tar.bz2"
        self.host_deps = ["musl-tools"]
        self.files = {
            "/bin/busybox": "busybox_unstripped"
        }

        self.symlinks = {}
        for cmd in COMMANDS:
            self.symlinks[f"/bin/{cmd}"] = "/bin/busybox"

    def build(self):
        self.run("ln -s /usr/bin/ar /usr/bin/musl-ar")
        self.run("ln -s /usr/bin/strip /usr/bin/musl-strip")
        self.make("allnoconfig")
        self.set_kconfig("CROSS_COMPILER_PREFIX", "musl-")
        self.set_kconfig("STATIC", True)
        self.set_kconfig("ECHO", True)
        self.set_kconfig("CAT", True)
        self.set_kconfig("UNAME", True)
        self.set_kconfig("ASH", True)
        self.set_kconfig("ASH_OPTIMIZE_FOR_SIZE", True)
        self.set_kconfig("NSLOOKUP", True)
        self.set_kconfig("VERBOSE_RESOLUTION_ERRORS", True)
        self.set_kconfig("DEBUG", True)
        self.make()
