#!/bin/sh
set -ue

if [ ! -d kerla ]; then
    git clone https://github.com/nuta/kerla
fi

cd kerla
git pull

curl https://sh.rustup.rs -sSf | sh -s -- -y
source $HOME/.cargo/env

rustup override set nightly
rustup component add rust-src

mkdir build
touch build/kerla.initramfs
make src-docs
mv target/doc ../public
