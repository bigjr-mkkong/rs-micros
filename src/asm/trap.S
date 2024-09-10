.option norvc
.section .text
.global s_trap_vector
s_trap_vector:

# Disable interrupt
    csrrc   zero, sie, 0x8

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

# Save FP regs
    #fsd     f0,  32 * 8(t0)
    #fsd     f1,  33 * 8(t0)
    #fsd     f2,  34 * 8(t0)
    #fsd     f3,  35 * 8(t0)
    #fsd     f4,  36 * 8(t0)
    #fsd     f5,  37 * 8(t0)
    #fsd     f6,  38 * 8(t0)
    #fsd     f7,  39 * 8(t0)
    #fsd     f8,  40 * 8(t0)
    #fsd     f9,  41 * 8(t0)
    #fsd     f10, 42 * 8(t0)
    #fsd     f11, 43 * 8(t0)
    #fsd     f12, 44 * 8(t0)
    #fsd     f13, 45 * 8(t0)
    #fsd     f14, 46 * 8(t0)
    #fsd     f15, 47 * 8(t0)
    #fsd     f16, 48 * 8(t0)
    #fsd     f17, 49 * 8(t0)
    #fsd     f18, 50 * 8(t0)
    #fsd     f19, 51 * 8(t0)
    #fsd     f20, 52 * 8(t0)
    #fsd     f21, 53 * 8(t0)
    #fsd     f22, 54 * 8(t0)
    #fsd     f23, 55 * 8(t0)
    #fsd     f24, 56 * 8(t0)
    #fsd     f25, 57 * 8(t0)
    #fsd     f26, 58 * 8(t0)
    #fsd     f27, 59 * 8(t0)
    #fsd     f28, 60 * 8(t0)
    #fsd     f29, 61 * 8(t0)
    #fsd     f30, 62 * 8(t0)
    #fsd     f31, 63 * 8(t0)



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

# Save FP regs
    #fld     f0,  32 * 8(t0)
    #fld     f1,  33 * 8(t0)
    #fld     f2,  34 * 8(t0)
    #fld     f3,  35 * 8(t0)
    #fld     f4,  36 * 8(t0)
    #fld     f5,  37 * 8(t0)
    #fld     f6,  38 * 8(t0)
    #fld     f7,  39 * 8(t0)
    #fld     f8,  40 * 8(t0)
    #fld     f9,  41 * 8(t0)
    #fld     f10, 42 * 8(t0)
    #fld     f11, 43 * 8(t0)
    #fld     f12, 44 * 8(t0)
    #fld     f13, 45 * 8(t0)
    #fld     f14, 46 * 8(t0)
    #fld     f15, 47 * 8(t0)
    #fld     f16, 48 * 8(t0)
    #fld     f17, 49 * 8(t0)
    #fld     f18, 50 * 8(t0)
    #fld     f19, 51 * 8(t0)
    #fld     f20, 52 * 8(t0)
    #fld     f21, 53 * 8(t0)
    #fld     f22, 54 * 8(t0)
    #fld     f23, 55 * 8(t0)
    #fld     f24, 56 * 8(t0)
    #fld     f25, 57 * 8(t0)
    #fld     f26, 58 * 8(t0)
    #fld     f27, 59 * 8(t0)
    #fld     f28, 60 * 8(t0)
    #fld     f29, 61 * 8(t0)
    #fld     f30, 62 * 8(t0)
    #fld     f31, 63 * 8(t0)

    csrrw   t0, sscratch, t0

    csrrs   zero, sie, 0x8

    sret
