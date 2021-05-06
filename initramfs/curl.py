from . import Package


class Curl(Package):
    def __init__(self):
        super().__init__()
        self.name = "curl"
        self.version = "7.76.1"
        self.url = f"https://curl.se/download/curl-{self.version}.tar.xz"
        self.host_deps = ["musl-tools"]
        self.files = {
            "/bin/curl": "src/curl",
        }

    def build(self):
        self.run([
            "./configure",
            "CC=musl-gcc",
            "--disable-shared",
            "--disable-pthreads",
            "--disable-threaded-resolver",
            "--disable-rtsp",
            "--disable-alt-svc",
            "--disable-libcurl-option",
            "--disable-telnet",
            "--disable-gopher",
            "--disable-dict",
            "--disable-file",
            "--disable-ftp",
            "--disable-tftp",
            "--disable-imap",
            "--disable-pop3",
            "--disable-smtp",
            "--disable-mqtt",
            "--disable-unix-sockets",
        ])
        self.make(["curl_LDFLAGS=-all-static"])
