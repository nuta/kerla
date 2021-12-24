#!/usr/bin/env python3
#
#  Usage: ./tools/create-release.py v0.1.0
#
import argparse
import os
import sys
import re
import subprocess

def main():
    parser = argparse.ArgumentParser(description="Creates a new release")
    parser.add_argument("release", help="The release version (e.g. 'v0.1.0').")
    args = parser.parse_args()

    prev_tag = subprocess.check_output(["git", "tag", "--sort=-creatordate"], text=True).splitlines()[0]
    next_tag = args.release

    log = subprocess.check_output(["git", "shortlog", f"{prev_tag}..HEAD"], text=True)
    print("*")
    print(f"*  Previous release: {prev_tag}")
    print(f"*  Next release:     {next_tag}")
    print("*")
    print()

    if not next_tag.startswith("v"):
        sys.exit("The next release must start with 'v'.")

    if input(f"Is this correct? (y/n) ").lower() != "y":
        print("Aborting.")
        return

    print("==> Updating crate versions...")
    for root, dirs, files in os.walk("."):
        for basename in files:
            if basename != "Cargo.toml":
                continue
            path = os.path.join(root, basename)
            body = open(path).read()
            if "[package]" not in body:
                continue

            new_ver = f"version = \"{next_tag.replace('v', '')}\""
            body = re.sub(f"^version\s+=\s+\"[^\"]+?\"", new_ver, body, count=1, flags=re.MULTILINE)
            if new_ver not in body:
                sys.exit(f"failed update the version in {path}")
            open(path, "w").write(body)

    print(f"==> Building the kernel")
    subprocess.run(["make", "RELEASE=1", "build"], check=True)

    print(f"==> Creating {next_tag}.log")
    open(f"{next_tag}.log", "w").write(log)

    print(f"==> Creating a commit for {next_tag}")
    subprocess.run(["git", "add", "."], check=True)
    subprocess.run(["git", "commit", "-m", next_tag], check=True)
    subprocess.run(["git", "tag", next_tag], check=True)

if __name__ == "__main__":
    main()
