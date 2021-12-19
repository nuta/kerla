#!/usr/bin/env python3
import argparse
import sys
import struct

START_MAKER = b"__SYMBOL_TABLE_START__"
END_MAKER = b"__SYMBOL_TABLE_END__"
SYMBOL_TABLE_MAGIC = 0xbeefbeef
SYMBOL_MAX_LEN = 55


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("symbols_file")
    parser.add_argument("executable")
    args = parser.parse_args()

    # Locate the symbol table in the executable.
    image = open(args.executable, "rb").read()
    offset = image.find(START_MAKER)
    offset_end = image.find(END_MAKER)
    if offset < 0 or offset_end < 0:
        sys.exit("embed-symbol-table.py: failed to locate the symbol table")
    if image.find(START_MAKER, offset + 1) >= 0:
        print(hex(offset), hex(image.find(START_MAKER, offset + 1)))
        sys.exit(
            "embed-symbol-table.py: found multiple empty symbol tables (perhaps because "
            + "START_MAKER is not sufficiently long to be unique?)")

    # Parse the nm output and extract symbol names and theier addresses.
    symbols = {}
    for line in open(args.symbols_file).read().strip().split("\n"):
        cols = line.split(" ", 1)
        try:
            addr = int(cols[0], 16)
        except ValueError:
            continue
        name = cols[1]
        symbols[addr] = name

    symbols = sorted(symbols.items(), key=lambda s: s[0])

    # Build a symbol table.
    symbol_table = struct.pack("<IIQ", SYMBOL_TABLE_MAGIC, len(symbols), 0)
    for addr, name in symbols:
        if len(name) <= SYMBOL_MAX_LEN:
            truncated_name = name[:55]
        else:
            prefix_len = SYMBOL_MAX_LEN // 2
            suffix_len = SYMBOL_MAX_LEN - len("...") - prefix_len
            truncated_name = name[:prefix_len] + "..." + name[-suffix_len:]
            assert len(truncated_name) == SYMBOL_MAX_LEN
        symbol_table += struct.pack("<Q56s", addr,
                                    bytes(truncated_name, "ascii"))

    max_size = offset_end - offset
    if len(symbol_table) > max_size:
        sys.exit(
            f"embed-symbol-table.py: Too many symbols; please expand the symbol table area (max_size={max_size / 1024}KiB, created={len(symbol_table) / 1024}KiB)"
        )

    # Embed the symbol table.
    image = image[:offset] + symbol_table + image[offset + len(symbol_table):]
    open(args.executable, "wb").write(image)


if __name__ == "__main__":
    main()
