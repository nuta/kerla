#!/bin/sh
set -ue

executable="$1"
$NM $executable | rustfilt | awk '{ $2=""; print $0 }' > $executable.symbols
$PYTHON3 ../tools/embed-symbol-table.py $executable.symbols $executable
$PYTHON3 ../tools/run-qemu.py --arch $ARCH $executable
