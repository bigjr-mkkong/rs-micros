.option norvc

.section .data

.section .text.init
.global _start
_start:
	csrr	t0, mhartid
	bnez	t0, 4f
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
	la		sp, _stack_start
    #set mstatus
	li		t0, (0b11 << 11) # | (1 << 13)# | (1 << 3)
	csrw	mstatus, t0
    #set mepc
	la		t1, eh_func_kinit
	csrw	mepc, t1
    #set mtvec
	la		t2, asm_trap_vector
	csrw	mtvec, t2

	# li		t3, (1 << 3) | (1 << 7) | (1 << 11)
	# csrw	mie, t3

	la		ra, 3f

	mret

3:
    csrw pmpaddr0, zero
    li t0, 7 | (1 << 3)
    csrw pmpcfg0, t0

  	li		t0, (1 << 8) | (1 << 5)
  	csrw	sstatus, t0
 
   	la		t1, eh_func_kmain
   	csrw	sepc, t1
  	
 	li		t2, (1 << 1) | (1 << 5) | (1 << 9)
 	csrw	mideleg, t2
 
 	csrw	sie, t2
 
 	la		t3, asm_trap_vector
 	csrw	stvec, t3
 
 	csrw	satp, a0
 
 	sfence.vma
 
 	sret

    
4:
5:
	nop
	j		5b
