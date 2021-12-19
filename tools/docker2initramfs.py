#!/usr/bin/env python3
import argparse
import subprocess
import sys
import tempfile
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(
        description="Converts a Docker image into cpio (initramfs format).")
    parser.add_argument("outfile", help="The output path.")
    parser.add_argument("image",
                        help="The docker image name (e.g. python:slim).")
    args = parser.parse_args()

    container_id = f"docker-initramfs-tmp"
    try:
        subprocess.run(["docker", "rm", container_id],
                       stdout=subprocess.DEVNULL,
                       stderr=subprocess.DEVNULL,
                       check=False)
        subprocess.run(
            ["docker", "create", "--name", container_id, "-t", args.image],
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=True)
        with tempfile.NamedTemporaryFile() as temp_file:
            temp_file = Path(temp_file.name)
            subprocess.run(
                ["docker", "export", f"--output={temp_file}", container_id],
                stderr=subprocess.STDOUT,
                check=True)
            with tempfile.TemporaryDirectory() as temp_dir:
                temp_dir = Path(temp_dir)
                subprocess.run(["tar", "xf", temp_file],
                               cwd=temp_dir,
                               check=True)

                # XXX: This is a hack to get around the fact that the Docker overrides
                #      the /etc/resolv.conf file.
                (temp_dir / "etc" /
                 "resolv.conf").write_text("nameserver 1.1.1.1")

                filelist = list(
                    map(lambda p: "./" + str(p.relative_to(temp_dir)),
                        temp_dir.glob("**/*")))
                subprocess.run(
                    ["cpio", "--create", "--format=newc"],
                    input="\n".join(filelist).encode("ascii"),
                    stdout=open(args.outfile, "wb"),
                    stderr=subprocess.PIPE,
                    cwd=temp_dir,
                )
    except subprocess.CalledProcessError as e:
        sys.exit(
            f"{e.stdout.decode('utf-8', 'backslashreplace')}\n\nError: failed to export {args.image}"
        )
    finally:
        subprocess.run(["docker", "rm", container_id],
                       stdout=subprocess.DEVNULL,
                       stderr=subprocess.DEVNULL,
                       check=False)


if __name__ == "__main__":
    main()
