.option norvc

.section .data

.section .text.init
.global _start
_start:
	csrr	t0, mhartid
	bnez	t0, 3f
	csrw	satp, zero

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
	la		sp, _stack
    #set mstatus
	li		t0, (0b11 << 11) # | (1 << 7) | (1 << 3)
	csrw	mstatus, t0
    #set mepc
	la		t1, kmain
	csrw	mepc, t1
    #set mtvec
	la		t2, asm_trap_vector
	csrw	mtvec, t2

	li		t3, (1 << 3) | (1 << 7) | (1 << 11)
	csrw	mie, t3

	la		ra, 4f

	mret
3:
4:
	nop
	j		4b


