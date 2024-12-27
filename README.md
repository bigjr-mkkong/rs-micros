## What is it
(Will be) A minix like riscv64gc micro kernel implementation, which focus on extensibility from hardware

## How to run
First configure rust:
```
rustup default nightly
rustup target add riscv64gc-unknown-none-elf
cargo install cargo-binutils
```
Then, make sure gcc riscv64 cross compiler toolchain and qemu riscv64 are installed, after that execute

```
make run
```

to boot the kernel.

Also, you may need to change $(PREFIX) variable in Makefile if your toolchain is different from the default one 

## How to debug
Run
```
make debug
```
To hang qemu before receive gdb client connection

## Current Progress
  - [x] Kernel Loader
  - [x] Uart (NS16550 compatible)
  - [x] Multi-core safety Page Allocator(naive one)
  - [x] VM under S-mode
  - [x] Trap frame
  - [x] CLINT Timer
  - [x] PLIC
  - [x] Small-object allocator(freelist version)
  - [x] Kthread
  - [x] Ecall from kthread
  - [x] Task pool & round-robin scheduler

  **. . .**

## TODO
There are many TODOs in src code, but here are some general things:
  - [x] ISA abstraction(Use `riscv` crate replace most inline asm)
  - [x] Proper logging system for multi-core
  - [x] CPU dumper inside trap code
  - [ ] `fsd` and `fld` would report illegal instruction error
  - [x] Slub(or any kinds of small object allocator) allocator for kheap
  - [x] Kernel threads
  - [ ] kernel semaphore
