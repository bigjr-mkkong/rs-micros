.option norvc

.section .data

.section .text.init
.global _start
_start:
	csrw	satp, zero
	csrr	t0, mhartid
# Set non-bsp cores to dead loop
	bnez	t0, 4f

.option push
.option norelax
	la		gp, _global_pointer
.option pop
    #Clear bss
	la 		a0, _bss_start
	la		a1, _bss_end
	bgeu	a0, a1, 2f
1:
	sd		zero, (a0)
	addi	a0, a0, 8
	bltu	a0, a1, 1b
2:
	la		sp, _stack_start
    #enable M-mode and fs = 0b11
	li		t0, (0b11 << 11) | (0b11 << 13) # | (1 << 13)# | (1 << 3)
	csrw	mstatus, t0
    #set mepc
	la		t1, eh_func_kinit
	csrw	mepc, t1
    #set mtvec
	la		t2, m_trap_vector
	csrw	mtvec, t2

	la		ra, 3f

    csrw    pmpaddr0, zero
    li      t0, 7 | (1 << 3)
    csrw    pmpcfg0, t0

#switch to M-mode
	mret

3:

#switch to S-mode
    mret

    
4: # other cpu(s) will arrive here

    #allocate per-cpu stack space
    la      sp, _stack_start
    li      t0, 0x10000
    csrr    a0, mhartid
    mul     t0, t0, a0
    sub     sp, sp, t0


    li      t0, 0b11 << 11 | (1 << 7)
    csrw    mstatus, t0

    li      t3, 1 << 3
    csrw    mie, t3

    la      t1, eh_func_kinit_nobsp
    csrw    mepc, t1

    la      t2, m_trap_vector
    csrw    mtvec, t2

    la      ra, 5f
    mret
    
5:
	nop
	j		5b
