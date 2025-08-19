#!/bin/bash

sudo apt update -y
sudo apt upgrade -y
sudo apt install -y software-properties-common gcc-riscv64-linux-gnu

rustup default nightly
rustup target add riscv64gc-unknown-none-elf 
rustup component add rustfmt

cargo clean
cargo install cargo-binutils 

if [ ! -d "qemu" ]; then
    sudo bash "./scripts/install-qemu.sh"
fi