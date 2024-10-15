## What is it
(Will be) A minix like riscv64gc micro kernel implementation, which focus on extensibility from hardware

## How to run
Make sure gcc riscv64 cross compiler toolchain and qemu riscv64 are installed, then execute
```
make run
```
to boot the kernel. (You might need to run mk-disk script first)

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

  **. . .**

## TODO
There are many TODOs in src code, but here are some general things:
  - [x] ISA abstraction(Use `riscv` crate replace most inline asm)
  - [x] Proper logging system for multi-core
  - [ ] CPU dumper inside trap code
  - [ ] `fsd` and `fld` would report illegal instruction error
  - [ ] Slub(or any kinds of small object allocator) allocator for kheap
