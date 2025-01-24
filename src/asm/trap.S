.option norvc
.section .text
.global s_trap_vector
.global m_trap_vector
s_trap_vector:
# sscratch keep the current-running task's trap frame, it lies in kmem space
# and is safe to access through physical memory
    csrrw   s1, sscratch, s1

    #disable memory translation
    csrw    satp, x0 
    sfence.vma

# Save GP regs
    sd      x0,  0   * 8(s1)
    sd      x1,  1   * 8(s1)
    sd      x2,  2   * 8(s1)
    sd      x3,  3   * 8(s1)
    sd      x4,  4   * 8(s1)
    sd      x5,  5   * 8(s1)
    sd      x6,  6   * 8(s1)
    sd      x7,  7   * 8(s1)
    sd      x8,  8   * 8(s1)
    sd      x10, 10  * 8(s1)
    sd      x11, 11  * 8(s1)
    sd      x12, 12  * 8(s1)
    sd      x13, 13  * 8(s1)
    sd      x14, 14  * 8(s1)
    sd      x15, 15  * 8(s1)
    sd      x16, 16  * 8(s1)
    sd      x17, 17  * 8(s1)
    sd      x18, 18  * 8(s1)
    sd      x19, 19  * 8(s1)
    sd      x20, 20  * 8(s1)
    sd      x21, 21  * 8(s1)
    sd      x22, 22  * 8(s1)
    sd      x23, 23  * 8(s1)
    sd      x24, 24  * 8(s1)
    sd      x25, 25  * 8(s1)
    sd      x26, 26  * 8(s1)
    sd      x27, 27  * 8(s1)
    sd      x28, 28  * 8(s1)
    sd      x29, 29  * 8(s1)
    sd      x30, 30  * 8(s1)
    sd      x31, 31  * 8(s1)

# switch back original s1(x9) value
    csrr    s2, sscratch
    sd      s2, 8 * 8(s1)

    csrrw   s1, sscratch, s1
    call    which_cpu
    csrrw   s1, sscratch, s1

    move    a3, a0

    csrr    a0, sepc
    csrr    a1, stval
    csrr    a2, scause
    csrr    a4, sstatus
    move    a5, t0

    ld      sp, 65 * 8(s1)

    csrrw   s1, sscratch, s1
    call    s_trap
    csrrw   s1, sscratch, s1

    csrw    sepc, a0

# Load back task's satp value
    ld      x1,  64  * 8(s1)
    csrw    satp, x1

# Load GP regs
    ld      x0,  0   * 8(s1)
    ld      x1,  1   * 8(s1)
    ld      x2,  2   * 8(s1)
    ld      x3,  3   * 8(s1)
    ld      x4,  4   * 8(s1)
    ld      x5,  5   * 8(s1)
    ld      x6,  6   * 8(s1)
    ld      x7,  7   * 8(s1)
    ld      x8,  8   * 8(s1)
    ld      x10, 10  * 8(s1)
    ld      x11, 11  * 8(s1)
    ld      x12, 12  * 8(s1)
    ld      x13, 13  * 8(s1)
    ld      x14, 14  * 8(s1)
    ld      x15, 15  * 8(s1)
    ld      x16, 16  * 8(s1)
    ld      x17, 17  * 8(s1)
    ld      x18, 18  * 8(s1)
    ld      x19, 19  * 8(s1)
    ld      x20, 20  * 8(s1)
    ld      x21, 21  * 8(s1)
    ld      x22, 22  * 8(s1)
    ld      x23, 23  * 8(s1)
    ld      x24, 24  * 8(s1)
    ld      x25, 25  * 8(s1)
    ld      x26, 26  * 8(s1)
    ld      x27, 27  * 8(s1)
    ld      x28, 28  * 8(s1)
    ld      x29, 29  * 8(s1)
    ld      x30, 30  * 8(s1)
    ld      x31, 31  * 8(s1)

    csrrw   s1, sscratch, s1

#refresh satp
    sfence.vma
    sret


m_trap_vector:
#KERNEL_TRAP_FRAME addr is inside mscratch
    csrrw   s1, mscratch, s1
    beqz    s1, paniker

# Save GP regs
    sd      x0,  0   * 8(s1)
    sd      x1,  1   * 8(s1)
    sd      x2,  2   * 8(s1)
    sd      x3,  3   * 8(s1)
    sd      x4,  4   * 8(s1)
    sd      x5,  5   * 8(s1)
    sd      x6,  6   * 8(s1)
    sd      x7,  7   * 8(s1)
    sd      x8,  8   * 8(s1)
    sd      x10, 10  * 8(s1)
    sd      x11, 11  * 8(s1)
    sd      x12, 12  * 8(s1)
    sd      x13, 13  * 8(s1)
    sd      x14, 14  * 8(s1)
    sd      x15, 15  * 8(s1)
    sd      x16, 16  * 8(s1)
    sd      x17, 17  * 8(s1)
    sd      x18, 18  * 8(s1)
    sd      x19, 19  * 8(s1)
    sd      x20, 20  * 8(s1)
    sd      x21, 21  * 8(s1)
    sd      x22, 22  * 8(s1)
    sd      x23, 23  * 8(s1)
    sd      x24, 24  * 8(s1)
    sd      x25, 25  * 8(s1)
    sd      x26, 26  * 8(s1)
    sd      x27, 27  * 8(s1)
    sd      x28, 28  * 8(s1)
    sd      x29, 29  * 8(s1)
    sd      x30, 30  * 8(s1)
    sd      x31, 31  * 8(s1)

# switch back original s1(x9) value
    csrr    s2, mscratch
    sd      s2, 9 * 8(s1)

    csrr    a0, mepc
    csrr    a1, mtval
    csrr    a2, mcause
    csrr    a3, mhartid
    csrr    a4, mstatus
    move    a5, s1

    ld      sp, 65 * 8(s1)
    call    m_trap

    csrw    mepc, a0

# Ignore zero register
# Load GP regs
    ld      x0,  0   * 8(s1)
    ld      x1,  1   * 8(s1)
    ld      x2,  2   * 8(s1)
    ld      x3,  3   * 8(s1)
    ld      x4,  4   * 8(s1)
    ld      x5,  5   * 8(s1)
    ld      x6,  6   * 8(s1)
    ld      x7,  7   * 8(s1)
    ld      x8,  8   * 8(s1)
    ld      x10, 10  * 8(s1)
    ld      x11, 11  * 8(s1)
    ld      x12, 12  * 8(s1)
    ld      x13, 13  * 8(s1)
    ld      x14, 14  * 8(s1)
    ld      x15, 15  * 8(s1)
    ld      x16, 16  * 8(s1)
    ld      x17, 17  * 8(s1)
    ld      x18, 18  * 8(s1)
    ld      x19, 19  * 8(s1)
    ld      x20, 20  * 8(s1)
    ld      x21, 21  * 8(s1)
    ld      x22, 22  * 8(s1)
    ld      x23, 23  * 8(s1)
    ld      x24, 24  * 8(s1)
    ld      x25, 25  * 8(s1)
    ld      x26, 26  * 8(s1)
    ld      x27, 27  * 8(s1)
    ld      x28, 28  * 8(s1)
    ld      x29, 29  * 8(s1)
    ld      x30, 30  * 8(s1)
    ld      x31, 31  * 8(s1)

    csrrw   s1, mscratch, s1

    mret
