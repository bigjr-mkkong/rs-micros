#!/bin/bash

sudo apt install -y python3 python3-venv python3-sphinx ninja-build

cd /workspaces/rs-micros
rm -rf qemu

if [ ! -d "qemu-repo" ]; then
    git clone https://github.com/qemu/qemu qemu-repo
fi

cd qemu-repo
git pull https://github.com/qemu/qemu.git master
./configure --target-list=riscv64-softmmu --prefix=/workspaces/rs-micros/qemu
make -j $(nproc)
sudo make install
cd ..