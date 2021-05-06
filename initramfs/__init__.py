#!/usr/bin/env python3
from glob import glob
import os
import re
import subprocess
import shutil
import shlex
import sys
import math
import tempfile
from pathlib import Path
from importlib import import_module
import argparse
import struct

BUILTIN_PACKAGES = [
    "build-essential", "curl", "sed"
]


class Package:
    def __init__(self):
        self.version = None
        self.url = None
        self.host_deps = None
        self.patch_id = 1
        self.dockerfile = []
        self.files = {}
        self.symlinks = {}
        self.added_files = {}

    def build(self):
        pass

    def run(self, argv):
        if type(argv) == str:
            self.dockerfile.append(f"RUN {argv}")
        else:
            self.dockerfile.append(
                f"RUN {' '.join([shlex.quote(arg) for arg in argv])}")

    def env(self, key, value):
        escaped = value.replace("\"", "\\\"")
        self.dockerfile.append([f"ENV {key}={escaped}"])

    def patch(self, patch):
        patch_file = f"__build_{self.patch_id}__.patch"
        self.add_file(patch_file, patch)
        self.run(["sh", "-c", f"patch --ignore-whitespace -p1 < {patch_file}"])
        self.patch_id += 1

    def make(self, cmd=None):
        if cmd:
            self.run(["make", f"-j{num_cpus()}", cmd])
        else:
            self.run(["make", f"-j{num_cpus()}"])

    def add_file(self, path, content):
        self.added_files[path] = content

    def set_kconfig(self, key, value):
        if type(value) == bool:
            value_str = "y" if value else "n"
        else:
            value_str = f'\\"{value}\\"'
        replace_with = f"CONFIG_{key}={value_str}"
        self.run(
            f"sh -c \""
            + f"sed -i -e 's/# CONFIG_{key} is not set/{replace_with}/' .config;"
            + f"sed -i -e 's/[# ]*CONFIG_{key}=.*/{replace_with}/' .config;\"")

    def generate_dockerfile(self):
        lines = [
            "FROM ubuntu:20.04",
            f"RUN apt-get update && apt-get install -qy {' '.join(BUILTIN_PACKAGES)}"
        ]

        if self.host_deps:
            lines.append(f"RUN apt-get install -qy {' '.join(self.host_deps)}")

        if self.url:
            ext = Path(self.url).suffix
            if ext == ".tar":
                tarball = "tarball.tar"
            elif ext in [".gz", ".bz2", ".xz"]:
                tarball = f"tarball.tar{ext}"
            else:
                raise Exception(
                    f"unknown file extension in the url: {self.url}")
            lines.append(f"RUN curl -fsSL --output {tarball} \"{self.url}\"")
            lines.append(
                f"RUN mkdir /build && tar xf {tarball} --strip-components=1 -C /build")

        for path, content in self.added_files.items():
            dst_path = os.path.join("/build", path.lstrip("/"))
            tmp_path = os.path.join("add_files", path.lstrip("/"))
            Path(tmp_path).parent.mkdir(parents=True, exist_ok=True)
            if type(content) is str:
                open(tmp_path, "w").write(content)
            else:
                open(tmp_path, "wb").write(content)
            lines.append(f"ADD {tmp_path} {dst_path}")

        lines.append("WORKDIR /build")
        lines += self.dockerfile
        return "\n".join(lines)


def num_cpus():
    return 16


all_symlinks = {}


def build_package(root_dir: Path, pkg):
    global all_symlinks
    root_dir = root_dir.absolute()
    cwd = os.getcwd()
    with tempfile.TemporaryDirectory() as tempdir:
        os.chdir(tempdir)
        # Prepare a Dockerfile for the package build.
        dockerfile = pkg.generate_dockerfile()
        open("Dockerfile", "w").write(dockerfile)

        container_id = f"penguin-{pkg.name}-container"
        print(
            f"  \x1b[1;96m{'DOCKER':>8}\x1b[0m  \x1b[1;m{pkg.name}\x1b[0m")
        try:
            # Build the package in Docker.
            subprocess.run(
                ["docker", "build", "-t", f"penguin-{pkg.name}", "."], cwd=tempdir,
                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=True)

            # Remove the exisiting container with the same name.
            subprocess.run(["docker", "rm", container_id],
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

            # Launch the container with that name.
            subprocess.run(
                ["docker", "run", "--name", container_id,
                    "-t", f"penguin-{pkg.name}", "/bin/true"],
                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=True)
        except subprocess.CalledProcessError as e:
            sys.exit(
                f"{e.stdout.decode('utf-8', 'backslashreplace')}\n\nError: failed to build {pkg.name}")

        # Copy files from the container.
        total_size = 0
        for dst, src in pkg.files.items():
            dst = root_dir.joinpath(dst.lstrip("/"))
            dst.parent.mkdir(parents=True, exist_ok=True)
            if not src.startswith("/"):
                src = "/build/" + src
            subprocess.run(
                ["docker", "cp", f"{container_id}:{src}", str(dst)], check=True)
            total_size += os.path.getsize(dst)

        subprocess.run(["docker", "rm", container_id],
                       stdout=subprocess.DEVNULL, check=True)

    all_symlinks.update(pkg.symlinks)
    os.chdir(cwd)


def camelcase(s):
    return "".join(t.title() for t in s.split("_"))


def get_packages():
    packages = {}
    sys.path.insert(0, os.getcwd())
    for path in glob("initramfs/*.py"):
        name = Path(path).stem
        if name in ["__init__", "__pycache__"]:
            continue

        mod = __import__(f"initramfs.{name}")
        klass = getattr(getattr(mod, name), camelcase(name))
        packages[name] = klass()
    return packages


def compute_tar_checksum(header):
    checksum = 0
    for byte in header:
        checksum += byte


def main():
    parser = argparse.ArgumentParser(
        description="The penguin initramfs build system.")
    parser.add_argument(
        "--build-dir", help="The build directory.", required=True)
    parser.add_argument("-o", dest="outfile", required=True)
    args = parser.parse_args()

    root_dir = Path(args.build_dir)
    shutil.rmtree(root_dir, ignore_errors=True)
    os.makedirs(root_dir, exist_ok=True)

    # Build packages.
    packages = get_packages()
    for pkg in packages.values():
        pkg.build()
        build_package(root_dir, pkg)

    # Add symlinks.
    for src, dst in all_symlinks.items():
        if src.startswith("/"):
            src = src[1:]
        os.symlink(dst, root_dir / src)

    print(f"  \x1b[1;96m{'CPIO':>8}\x1b[0m  \x1b[1;m{args.outfile}\x1b[0m")
    filelist = list(
        map(lambda p: "./" + str(p.relative_to(root_dir)), root_dir.glob("**/*")))
    cp = subprocess.run(
        ["cpio", "--create", "--format=newc"],
        input="\n".join(filelist).encode("ascii"),
        stdout=open(args.outfile, "wb"),
        stderr=subprocess.PIPE,
        cwd=root_dir,
        check=True,
    )

    if not cp.stderr.endswith(b" blocks\n"):
        print(cp.stderr)


if __name__ == "__main__":
    main()
