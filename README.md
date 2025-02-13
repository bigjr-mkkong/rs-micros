## What is it
(Will be) A minix like riscv64gc micro kernel implementation, which build from ground-up(no SBI) and just for fun reason

## How to run
First you need to configure your local rust toolchain by running following commands:
```
rustup default nightly
rustup target add riscv64gc-unknown-none-elf
cargo install cargo-binutils
```
Then, make sure gcc riscv64 cross compiler toolchain and qemu riscv64 are installed. After that execute

```
make run
```

to compile & build & boot kernel into qemu.

Also, you may need to change $(PREFIX) variable if your toolchain is different from the default one 

You can also run `make all` to just build binary into elf file or `make bitstream` to build raw bitstream
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
  - [x] Task pool & round-robin scheduler & context switch
  - [x] More tests on multi-core schedule
  - [x] Basic kthread sync primitives`(spawn(), join_all(), exit())`
  - [x] soft irq
  - [x] kthread semaphore
  - [Working...] ksemaphore stress test
  - [ ] User task
  - [ ] User syscall

  **. . .**

## TODO
There are many TODOs in src code, but here are some general things:
  - [x] ISA abstraction(Use `riscv` crate replace most inline asm)
  - [x] Proper logging system for multi-core
  - [x] CPU dumper inside trap code
  - [ ] `fsd` and `fld` would report illegal instruction error
  - [x] Slub(or any kinds of small object allocator) allocator for kheap
  - [x] Kernel threads
  - [ ] I am really interested on the idea of embed HW-malloc(like FALAFEL?) into this OS, and I already implemented a really nice-looking FFI between rust and C(which is the main reason of why I use C implemented small object allocator). I think it's not hard to just hookup allocator into this OS, but it may tooks a lot of time to fix compatibility issues depends on different kinds of dev boards.
  - [x] Dev-tree parser
  - [ ] Cross-platform interrupt based on device tree information
  - [x] High-intensity interrupt in a short amount of time will overflow kernel heap and cause UB on whole system. I need to implement kthread semaphore to solve this problem.
