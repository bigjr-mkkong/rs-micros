## What is it

A Minix-like `riscv64gc` microkernel built from the ground up, without an SBI.

## How to run

The build requires:

- a current Rust nightly toolchain with the bare-metal RISC-V target;
- a `riscv64-linux-gnu-` GCC/binutils cross toolchain; and
- `qemu-system-riscv64`.

Configure Rust with:

```
rustup default nightly
rustup target add riscv64gc-unknown-none-elf
```

Then build and boot the kernel with:

```
make run
```

The kernel boots four harts directly in QEMU's `virt` machine, switches them to
S-mode, starts their schedulers, and waits for UART input on hart 0. Type a
character to exercise the UART interrupt and semaphore wakeup path. Press
`Ctrl-a`, then `x`, to quit QEMU.

If the cross-toolchain uses a different prefix, override it on the command line,
for example `make PREFIX=riscv64-unknown-elf- run`.

Run `make all` to produce `os.elf`, or `make bitstream` to additionally produce
the raw `os.bin` image.

## Multicore ktask and semaphore test

Run the automated four-hart stress test with:

```
make test-multicore
```

The test runs two yielding ktasks per hart for 128 total scheduler records, then
passes 64 signals through a shared kernel semaphore, with each hart consuming
16 wakeups. Every record is printed through `Sprintln!`; the host validates the
per-hart distribution and checks that all 128 ktask records and all 64 semaphore
records are complete, non-interleaved lines. QEMU exits automatically when both
phases pass.

## How to debug

Run:

```
make debug
```

QEMU will wait for a GDB connection on TCP port 1234 before executing the first
instruction.

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
  - [x] ksemaphore stress test
  - [Working...] User task
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
