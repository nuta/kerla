#!/usr/bin/env python3
import argparse
import subprocess
import sys
import shlex
import json


def main():
    parser = argparse.ArgumentParser(
        description=
        "Determines the init argv specified in a Docker image (i.e. CMD).")
    parser.add_argument(
        "image",
        help=
        "The docker image name (e.g. python:slim or docker.io/library/python:alpine)."
    )
    args = parser.parse_args()

    try:
        stdout = subprocess.check_output(
            ["docker", "image", "inspect", args.image])
    except subprocess.CalledProcessError as e:
        sys.exit(
            f"{e.stdout.decode('utf-8', 'backslashreplace')}\n\nError: failed to inspect {args.image}"
        )

    data = json.loads(stdout)[0]["Config"]
    print(shlex.join(data["Cmd"]))


if __name__ == "__main__":
    main()
