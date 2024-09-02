.section .text
.global asm_trap_vector
asm_trap_vector:
    csrr    t0, mtval
    csrr    t1, mcause
    nop
    j asm_trap_vector
