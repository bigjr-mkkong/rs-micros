.option norvc                 # Disable RISC-V compressed instructions

.section .data
.equ    BSP,     0            # Define BSP (Boot Strap Processor) core ID as 0
.global cpu_early_block
cpu_early_block:    
    .dword      (1 << BSP)    # Bitmask for CPU early blocking (bit 0 set for BSP)

.global fdt_base
fdt_base:
    .dword      0             # Placeholder for Flattened Device Tree (FDT) pointer

.section .text.init
.global _start
_start:
    la      t0, fdt_base      # Load address of fdt_base into t0
    sd      a1, (t0)          # Store FDT pointer (in a1) to memory at fdt_base

    csrw    satp, zero        # Disable virtual memory translation (SATP = 0)

.option push
.option norelax
    la      gp, _global_pointer # Set GP before either BSP or secondary Rust code runs
.option pop

    csrr    t0, mhartid       # Read core ID into t0
    li      t1, BSP           # Load BSP value (0) into t1
    bne     t0, t1, 4f        # Secondary harts wait for BSP initialization at 4:

    # Clear .bss section
    la      a0, _bss_start    # Load BSS start address
    la      a1, _bss_end      # Load BSS end address
    bgeu    a0, a1, 2f        # If start >= end, skip clearing
    
1:
    sd      zero, (a0)        # Store 0 at current address
    addi    a0, a0, 8         # Move to next 8-byte slot
    bltu    a0, a1, 1b        # Loop if a0 < a1

2:
    # Setup stack for BSP
    la      sp, _stack_start  # Load initial stack address
    li      t0, 0x10000       # Stack size per core (64 KB)
    csrr    a0, mhartid       # Get core ID
    mul     t0, t0, a0        # Offset = core_id * stack size
    sub     sp, sp, t0        # Allocate per-core stack for BSP

    # Set mstatus: MPP = Machine mode, FS = enabled
    li      t0, (0b11 << 11) | (0b11 << 13)
    csrw    mstatus, t0

    # Set mepc to kernel init function
    la      t1, eh_func_kinit
    csrw    mepc, t1

    # Set trap vectors
    la      t2, m_trap_vector
    csrw    mtvec, t2
    la      t2, s_trap_vector
    csrw    stvec, t2

    # Give S-mode access to the full physical address space.  A TOR entry with
    # pmpaddr0 == 0 describes an empty range, so the old setup denied every
    # S-mode access as soon as mret left M-mode.
    li      t0, -1
    csrw    pmpaddr0, t0
    li      t0, 7 | (1 << 3)  # TOR, RWX permissions
    csrw    pmpcfg0, t0

    # Jump to eh_func_kinit() in M-mode via mret
    la      ra, 3f            # Update return address after eh_func_kinit() finish
    mret

3:
    # Jump to eh_func_kmain() in S-mode via mret
    csrr    a0, mhartid       # Read core ID
    mret                      # Jump to eh_func_kmain()


# Non-BSP CPUs enter here:
4:
    # Allocate stack space for Non-BSP
    la      sp, _stack_start
    li      t0, 0x10000
    csrr    a0, mhartid
    mul     t0, t0, a0
    sub     sp, sp, t0

    # Non-BSP early blocking
    li      t1, 1
    sll     t1, t1, a0        # t1 = 1 << core_id

7:
    ld      t2, cpu_early_block
    and     t2, t2, t1        # Check if BSP set our bit
    beqz    t2, 7b            # Loop until it’s set
    fence   r, rw              # Acquire BSP page-table and scheduler initialization

    # Set trap vectors
    la      t2, m_trap_vector
    csrw    mtvec, t2
    la      t2, s_trap_vector
    csrw    stvec, t2

    li      t0, 0b11 << 11 | (0b11 << 13)
    csrw    mstatus, t0

    la      t1, eh_func_kinit_nobsp
    csrw    mepc, t1

    li      t0, -1
    csrw    pmpaddr0, t0
    li      t0, 7 | (1 << 3)  # TOR, RWX permissions
    csrw    pmpcfg0, t0

    la      ra, 6f
    mret                      # Jump to eh_func_kinit_nobsp()

6:
    mret                      # Jump to eh_func_kmain_nobsp()
