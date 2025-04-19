#!/workspaces/rs-micros/scripts/install-qemu

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

# Doesn't work in my setup as $PATH gets reset all the time for some reason
# Commented for consistency - makefile is good enough
# export PATH="$PATH:/workspaces/rs-micros/qemu/bin"