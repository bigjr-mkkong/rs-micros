.option norvc

.section .data
.equ    BSP,     0
.global cpu_early_block
cpu_early_block:    
    .dword      (1 << BSP)

.global dtb_base
dtb_base:
    .dword      0

.section .text.init
.global _start
_start:
    la      t0, dtb_base
    sd      a1, (t0)
	csrw	satp, zero
	csrr	t0, mhartid
    li      t1, BSP
# Uncomment following line to part all non-bsp cores into inf loop
    bne     t0, t1, 5f
# Set non-bsp cores to dead loop
	bne	    t0, t1, 4f

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
    la      sp, _stack_start
    li      t0, 0x10000
    csrr    a0, mhartid
    mul     t0, t0, a0
    sub     sp, sp, t0

    #enable M-mode and fs = 0b11
	li		t0, (0b11 << 11) | (0b11 << 13) # | (1 << 7) | (1 << 13)# | (1 << 3)
	csrw	mstatus, t0

    #set mepc
	la		t1, eh_func_kinit
	csrw	mepc, t1

    #set mtvec
	la		t2, m_trap_vector
	csrw	mtvec, t2

    #set stvec
	la		t2, s_trap_vector
	csrw	stvec, t2

	la		ra, 3f

    csrw    pmpaddr0, zero
    li      t0, 7 | (1 << 3)
    csrw    pmpcfg0, t0

#switch to M-mode
	mret

3:

#bsp switch to S-mode
    mret

    
4: # other cpu(s) will arrive here

    #allocate per-cpu stack space
    la      sp, _stack_start
    li      t0, 0x10000
    csrr    a0, mhartid
    mul     t0, t0, a0
    sub     sp, sp, t0

    li      t1, 1
    sll     t1, t1, a0
#   Non-bsp will block until bsp kinit() finished
7:
    ld      t2, cpu_early_block
    and     t2, t2, t1
    beqz    t2, 7b

    #set mtvec
	la		t2, m_trap_vector
	csrw	mtvec, t2

    #set stvec
	la		t2, s_trap_vector
	csrw	stvec, t2

    li      t0, 0b11 << 11 | (0b11 << 13)
    csrw    mstatus, t0

    la      t1, eh_func_kinit_nobsp
    csrw    mepc, t1

    csrw    pmpaddr0, zero
    li      t0, 7 | (1 << 3)
    csrw    pmpcfg0, t0

    la      ra, 6f
    mret

6:
    mret
    
5:
	nop
	j		5b
