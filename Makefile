# / ___|  / ___|  / ___|
#| |  _  | |     | |    
#| |_| | | |___  | |___ 
# \____|  \____|  \____|

CC=riscv64-linux-gnu-g++
OBJDUMP=riscv64-linux-gnu-objdump

CFLAGS=-Wall -Wextra -pedantic -Wextra -O0 -std=c++17 -g
CFLAGS+=-static -ffreestanding -nostdlib -fno-rtti -fno-exceptions
CFLAGS+=-march=rv64gc -mabi=lp64d

INCLUDES=
LINKER_SCRIPT=-Tsrc/lds/virt.lds
TYPE=debug
RUST_TARGET=./target/riscv64gc-unknown-none-elf/$(TYPE)
LIBS=-L$(RUST_TARGET)
SOURCES_ASM=$(wildcard src/asm/*.S)
LIB= -lgcc -lrs_micros
OUT=os.elf

# / _ \  | ____| |  \/  | | | | |
#| | | | |  _|   | |\/| | | | | |
#| |_| | | |___  | |  | | | |_| |
# \__\_\ |_____| |_|  |_|  \___/ 
                                
QEMU=qemu-system-riscv64
MACH=virt
CPU=rv64
CPUS=4
MEM=128M
DRIVE=hdd.dsk

all:
	cargo build
	$(CC) $(CFLAGS) $(LINKER_SCRIPT) $(INCLUDES) -o $(OUT) $(SOURCES_ASM) $(LIBS) $(LIB)
	
run: all dump
	$(QEMU) -machine $(MACH) -cpu $(CPU) -smp $(CPUS) -m $(MEM)  -nographic -serial mon:stdio -bios none -kernel $(OUT) -drive if=none,format=raw,file=$(DRIVE),id=foo -device virtio-blk-device,scsi=off,drive=foo -s -S

dump: all
	$(OBJDUMP) -D $(OUT) > dump

.PHONY: clean
clean:
	cargo clean
	rm -f $(OUT)
	rm dump

