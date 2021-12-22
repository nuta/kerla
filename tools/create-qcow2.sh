#!/bin/bash
#
#  Usage: ./tools/create-qcow2.sh <kerla_elf> <qcow2_file>
#
set -ue
KERNEL_IMG=$1
OUTPUT_FILE=$2
MNT_DIR=$PWD/build/mnt

ignore_failure() {
    set +e
    $*
    set -e
}

progress() {
    echo
    echo -e "\x1b[94;1m==> $*\x1b[0m"
}

mkdir -p $MNT_DIR

progress "Creating a disk image"
dd if=/dev/zero of=build/disk.raw bs=512 count=131072
echo -e "n\np\n\n\n\na\nw" | fdisk build/disk.raw

ignore_failure sudo umount $MNT_DIR
ignore_failure sudo losetup -d /dev/loop32
ignore_failure sudo losetup -d /dev/loop33

progress "Populating the FAT filesystem"
sudo losetup /dev/loop33 build/disk.raw -o 1048576
sudo mkdosfs -F32 -f 2 /dev/loop33
sudo mount /dev/loop33 $MNT_DIR
sudo mkdir -p $MNT_DIR/boot/grub
sudo cp grub.cfg $MNT_DIR/boot/grub/grub.cfg
sudo cp $KERNEL_IMG $MNT_DIR/kerla.elf
sync; sync; sync

progress "Installing GRUB"
sudo losetup /dev/loop32 build/disk.raw
sudo grub-install --target=i386-pc \
    --root-directory=$MNT_DIR \
    --no-floppy \
    --modules="part_msdos fat multiboot2" \
    /dev/loop32

progress "Convert into qcow2"
qemu-img convert -f raw -O qcow2 build/disk.raw OUTPUT_FILE

sudo umount $MNT_DIR
sudo losetup -d /dev/loop32
sudo losetup -d /dev/loop33
