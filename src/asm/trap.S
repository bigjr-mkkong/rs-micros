.option norvc
.section .text
.global s_trap_vector
.global m_trap_vector
s_trap_vector:
#KERNEL_TRAP_FRAME addr is inside mscratch
    csrrw   t0, sscratch, t0

# Ignore zero register
# Save GP regs
    sd      x1,  0  * 8(t0)
    sd      x2,  1  * 8(t0)
    sd      x3,  2  * 8(t0)
    sd      x4,  3  * 8(t0)
    sd      x6,  5  * 8(t0)
    sd      x7,  6  * 8(t0)
    sd      x8,  7  * 8(t0)
    sd      x9,  8  * 8(t0)
    sd      x10, 9  * 8(t0)
    sd      x11, 10 * 8(t0)
    sd      x12, 11 * 8(t0)
    sd      x13, 12 * 8(t0)
    sd      x14, 13 * 8(t0)
    sd      x15, 14 * 8(t0)
    sd      x16, 15 * 8(t0)
    sd      x17, 16 * 8(t0)
    sd      x18, 17 * 8(t0)
    sd      x19, 18 * 8(t0)
    sd      x20, 19 * 8(t0)
    sd      x21, 20 * 8(t0)
    sd      x22, 21 * 8(t0)
    sd      x23, 22 * 8(t0)
    sd      x24, 23 * 8(t0)
    sd      x25, 24 * 8(t0)
    sd      x26, 25 * 8(t0)
    sd      x27, 26 * 8(t0)
    sd      x28, 27 * 8(t0)
    sd      x29, 28 * 8(t0)
    sd      x30, 29 * 8(t0)
    sd      x31, 30 * 8(t0)

# switch back original t0(x5) value
    csrr    t1, sscratch
    sd      t1, 4 * 8(t0)

    call    which_cpu
    move    a3, a0

    csrr    a0, sepc
    csrr    a1, stval
    csrr    a2, scause
    csrr    a4, sstatus
    move    a5, t0

    ld      sp, 65 * 8(t0)
    call    s_trap

    csrw    sepc, a0

# Ignore zero register
# Load GP regs
    ld      x1,  0  * 8(t0)
    ld      x2,  1  * 8(t0)
    ld      x3,  2  * 8(t0)
    ld      x4,  3  * 8(t0)
# x5(t0) has been used as base addr for TrapFram
    ld      x6,  5  * 8(t0)
    ld      x7,  6  * 8(t0)
    ld      x8,  7  * 8(t0)
    ld      x9,  8  * 8(t0)
    ld      x10, 9  * 8(t0)
    ld      x11, 10 * 8(t0)
    ld      x12, 11 * 8(t0)
    ld      x13, 12 * 8(t0)
    ld      x14, 13 * 8(t0)
    ld      x15, 14 * 8(t0)
    ld      x16, 15 * 8(t0)
    ld      x17, 16 * 8(t0)
    ld      x18, 17 * 8(t0)
    ld      x19, 18 * 8(t0)
    ld      x20, 19 * 8(t0)
    ld      x21, 20 * 8(t0)
    ld      x22, 21 * 8(t0)
    ld      x23, 22 * 8(t0)
    ld      x24, 23 * 8(t0)
    ld      x25, 24 * 8(t0)
    ld      x26, 25 * 8(t0)
    ld      x27, 26 * 8(t0)
    ld      x28, 27 * 8(t0)
    ld      x29, 28 * 8(t0)
    ld      x30, 29 * 8(t0)
    ld      x31, 30 * 8(t0)

    csrrw   t0, sscratch, t0

    sret


# TODO:
# This is not irq_save, this only disable interrupt
# If the interrupt was already disabled, it can re-enable interrupt when handler is done
# which is not what we are tring to do. We need another irq_save_cli sort of thing
# and an irq_restore
# I think it is safe to say, if the trap is taken from async, the mie is already set
# so we can safely clear & set here
# Otherwise for sync int, we have no idea about if MIE is set or not, so we need to save
# Like, irq_save before ecall and irq_restore after ecall
# Pagefault needs more thinking, but lets defer that one
m_trap_vector:
#KERNEL_TRAP_FRAME addr is inside mscratch
    csrrw   s1, mscratch, s1

# Ignore zero register
# Save GP regs
    sd      x1,  0  * 8(s1)
    sd      x2,  1  * 8(s1)
    sd      x3,  2  * 8(s1)
    sd      x4,  3  * 8(s1)
    sd      x5,  4  * 8(s1)
    sd      x6,  5  * 8(s1)
    sd      x7,  6  * 8(s1)
    sd      x8,  7  * 8(s1)
    sd      x10, 9  * 8(s1)
    sd      x11, 10 * 8(s1)
    sd      x12, 11 * 8(s1)
    sd      x13, 12 * 8(s1)
    sd      x14, 13 * 8(s1)
    sd      x15, 14 * 8(s1)
    sd      x16, 15 * 8(s1)
    sd      x17, 16 * 8(s1)
    sd      x18, 17 * 8(s1)
    sd      x19, 18 * 8(s1)
    sd      x20, 19 * 8(s1)
    sd      x21, 20 * 8(s1)
    sd      x22, 21 * 8(s1)
    sd      x23, 22 * 8(s1)
    sd      x24, 23 * 8(s1)
    sd      x25, 24 * 8(s1)
    sd      x26, 25 * 8(s1)
    sd      x27, 26 * 8(s1)
    sd      x28, 27 * 8(s1)
    sd      x29, 28 * 8(s1)
    sd      x30, 29 * 8(s1)
    sd      x31, 30 * 8(s1)

# switch back original s1(x9) value
    csrr    s2, mscratch
    sd      s2, 8 * 8(s1)

    csrr    a0, mepc
    csrr    a1, mtval
    csrr    a2, mcause
    csrr    a3, mhartid
    csrr    a4, mstatus
    move    a5, t0

    ld      sp, 65 * 8(s1)
    call    m_trap

    csrw    mepc, a0

# Ignore zero register
# Load GP regs
    ld      x1,  0  * 8(s1)
    ld      x2,  1  * 8(s1)
    ld      x3,  2  * 8(s1)
    ld      x4,  3  * 8(s1)
    ld      x5,  4  * 8(s1)
    ld      x6,  5  * 8(s1)
    ld      x7,  6  * 8(s1)
    ld      x8,  7  * 8(s1)
    ld      x10, 9  * 8(s1)
    ld      x11, 10 * 8(s1)
    ld      x12, 11 * 8(s1)
    ld      x13, 12 * 8(s1)
    ld      x14, 13 * 8(s1)
    ld      x15, 14 * 8(s1)
    ld      x16, 15 * 8(s1)
    ld      x17, 16 * 8(s1)
    ld      x18, 17 * 8(s1)
    ld      x19, 18 * 8(s1)
    ld      x20, 19 * 8(s1)
    ld      x21, 20 * 8(s1)
    ld      x22, 21 * 8(s1)
    ld      x23, 22 * 8(s1)
    ld      x24, 23 * 8(s1)
    ld      x25, 24 * 8(s1)
    ld      x26, 25 * 8(s1)
    ld      x27, 26 * 8(s1)
    ld      x28, 27 * 8(s1)
    ld      x29, 28 * 8(s1)
    ld      x30, 29 * 8(s1)
    ld      x31, 30 * 8(s1)

    csrrw   s1, mscratch, s1

    mret
