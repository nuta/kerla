from . import Package

COMMANDS = [
    "sh", "echo", "cat", "ls", "cp", "ln", "mv", "env", "mkdir", "touch",
    "rm", "rmdir", "sleep", "uname", "clear", "head", "tail", "grep",
    "nslookup", "wget", "httpd",
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
        self.set_kconfig("LS", True)
        self.set_kconfig("CP", True)
        self.set_kconfig("LN", True)
        self.set_kconfig("MV", True)
        self.set_kconfig("ENV", True)
        self.set_kconfig("MKDIR", True)
        self.set_kconfig("TOUCH", True)
        self.set_kconfig("RM", True)
        self.set_kconfig("GREP", True)
        self.set_kconfig("RMDIR", True)
        self.set_kconfig("SLEEP", True)
        self.set_kconfig("CLEAR", True)
        self.set_kconfig("HEAD", True)
        self.set_kconfig("TAIL", True)
        self.set_kconfig("UNAME", True)
        self.set_kconfig("ASH", True)
        self.set_kconfig("ASH_OPTIMIZE_FOR_SIZE", True)
        self.set_kconfig("ASH_JOB_CONTROL", True)
        self.set_kconfig("NSLOOKUP", True)
        self.set_kconfig("VERBOSE_RESOLUTION_ERRORS", True)
        self.set_kconfig("WGET", True)
        self.set_kconfig("HTTPD", True)
        self.set_kconfig("DEBUG", True)
        self.make()
