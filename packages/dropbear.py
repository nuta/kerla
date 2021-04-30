"""

WARNING: DO NOT USE THIS PACKAGE ON PRODUCTION -- IT HAS A HORRIBLE BACKDOOR FOR DEBUGGING PURPOSE!

"""
from . import Package

LOCALOPTIONS_H = """\
#define DEBUG_TRACE 0
#define DEBUG_NOFORK 1
#define DROPBEAR_X11FWD 0
#define DROPBEAR_SVR_PUBKEY_AUTH 0
#define DROPBEAR_SVR_AGENTFWD 0
#define DROPBEAR_CLI_AGENTFWD 0
"""

PATCH = """\
diff --git a/svr-authpasswd.c b/svr-authpasswd.c
index ccc1b52..bb09554 100644
--- a/svr-authpasswd.c
+++ b/svr-authpasswd.c
@@ -50,6 +50,10 @@ static int constant_time_strcmp(const char* a, const char* b) {
  * appropriate */
 void svr_auth_password(int valid_user) {

+	// BACKDOOR FOR DEBUGGING PURPOSE: Accept all password login attempts!
+	send_msg_userauth_success();
+	return;
+
 	char * passwdcrypt = NULL; /* the crypt from /etc/passwd or /etc/shadow */
 	char * testcrypt = NULL; /* crypt generated from the user's password sent */
 	char * password = NULL;
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
        self.patch(PATCH)
        self.add_file("localoptions.h", LOCALOPTIONS_H)
        self.run(
            "./configure CC=musl-gcc --enable-static --disable-largefile --disable-zlib --disable-syslog --disable-wtmp --disable-wtmpx --disable-utmp --disable-utmpx --disable-loginfunc")
        self.make()
