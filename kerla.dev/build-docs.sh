#!/bin/sh
set -ue

if [ ! -d kerla ]; then
    git clone https://github.com/nuta/kerla
fi

cd kerla
git pull
make docs
mv build/docs ../public/docs
