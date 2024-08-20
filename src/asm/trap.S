.section .text
.global asm_trap_vector
asm_trap_vector:
    csrr    t0, mepc
    addi    t0, t0, 8
    csrw    mepc, t0
    mret
