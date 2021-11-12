#!/usr/bin/env python3
import argparse
import os
from pathlib import Path

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out-dir", help="The output directory.")
    parser.add_argument("kexts", nargs="+", help="The list of kernel extensions (see kexts/*).")
    args = parser.parse_args()

    os.makedirs(args.out_dir, exist_ok=True)
    cargo_toml = """\
[package]
name = "kerla_kexts_loader"
description = "Kerla kernel extensions loader. Automatically generated."
version = "0.0.0"
edition = "2021"

[lib]
name = "kerla_kexts_loader"
path = "lib.rs"

[dependencies]
log = "0"
"""
    lib_rs = """\
#![no_std]

pub fn load_all() {
"""
    for kext in args.kexts:
        cargo_toml += f"{kext} = {{ path = \"{os.getcwd()}/exts/{kext}\" }}\n"
        lib_rs += f"    log::info!(\"kext: loading {kext}\");\n"
        lib_rs += f"    ::{kext}::init();\n"

    lib_rs += """
}
"""

    (Path(args.out_dir) / "Cargo.toml").write_text(cargo_toml)
    (Path(args.out_dir) / "lib.rs").write_text(lib_rs)

if __name__ == "__main__":
    main()
