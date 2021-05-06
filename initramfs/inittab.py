from . import Package

RC_STARTUP = """\
#!/bin/sh
dropbearkey -t ed25519 -f /tmp/dropbear_host_key
dropbear -F -B -r /tmp/dropbear_host_key -P /tmp/dropbear.pid

/bin/sh
"""

INITTAB = """\
::sysinit:/etc/init.d/rcS
::askfirst:/bin/sh
"""


class Inittab(Package):
    def __init__(self):
        super().__init__()
        self.name = "inittab"
        self.version = ""
        self.url = None
        self.host_deps = []
        self.files = {
            "/etc/inittab": "inittab",
            "/etc/init.d/rcS": "rcS",
            # FIXME: Use Busybox's init(1)
            "/sbin/init": "rcS",
        }

    def build(self):
        self.add_file("inittab", INITTAB)
        self.add_file("rcS", RC_STARTUP)
