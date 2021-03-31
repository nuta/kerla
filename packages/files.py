from . import Package
import os

FILES = {}
FILES["/dev/.keep"] = ""
FILES["/etc/banner"] = r"""
 _________________________________
< Rewrite in Rust ALL THE THINGS! >
 ---------------------------------
        \   ^__^
         \  (oo)\_______
            (__)\       )\/\
                ||----w |
                ||     ||
""".lstrip()
FILES["/etc/resolv.conf"] = """
nameserver 1.1.1.1
""".lstrip()


class Files(Package):
    def __init__(self):
        super().__init__()
        self.name = "files"
        self.version = ""
        self.url = None
        self.host_deps = []
        self.files = {path: path.lstrip("/") for path in FILES.keys()}

    def build(self):
        for path, content in FILES.items():
            self.add_file(path.lstrip("/"), content)
