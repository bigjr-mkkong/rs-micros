sudo apt update -y
sudo apt upgrade -y
sudo apt install -y software-properties-common

rustup default nightly
rustup target add riscv64gc-unknown-none-elf 
cargo install cargo-binutils 

sudo apt-get install -y gcc-riscv64-linux-gnu

if [ ! -d "qemu" ]; then
    sudo bash "scripts/install-qemu.sh"
fi