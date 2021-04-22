from . import Package

LOCALOPTIONS_H = """\
#define DEBUG_TRACE 0
#define DEBUG_NOFORK 1
#define DROPBEAR_X11FWD 0
#define DROPBEAR_SVR_AGENTFWD 0
#define DROPBEAR_CLI_AGENTFWD 0
"""


class Dropbear(Package):
    def __init__(self):
        super().__init__()
        self.name = "dropbear"
        self.version = "2020.81"
        self.url = f"https://matt.ucc.asn.au/dropbear/releases/dropbear-{self.version}.tar.bz2"
        self.host_deps = ["musl-tools"]
        self.files = {
            "/bin/dropbear": "dropbear",
            "/bin/dropbearkey": "dropbearkey",
        }

    def build(self):
        self.add_file("localoptions.h", LOCALOPTIONS_H)
        self.run(
            "./configure CC=musl-gcc --enable-static --disable-largefile --disable-zlib --disable-syslog --disable-wtmp --disable-wtmpx --disable-utmp --disable-utmpx --disable-loginfunc")
        self.make()
