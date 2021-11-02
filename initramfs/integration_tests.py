from . import Package
import os
from pathlib import Path

FILES = list(map(lambda f: Path("integration_tests") / f, os.listdir("integration_tests")))

class IntegrationTests(Package):
    def __init__(self):
        super().__init__()
        self.name = "integration_tests"
        self.version = "0.0.1"
        self.host_deps = []
        self.files = {"/tests/" + path.name: path.name for path in FILES }

    def build(self):
        for path in FILES:
            self.add_file(path.name, path.open().read())
