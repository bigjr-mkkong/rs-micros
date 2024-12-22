PREFIX=riscv64-linux-gnu-
CC=$(PREFIX)gcc

CFLAGS=-Wall -Wextra -pedantic -Wextra -O0 -g -c
# CFLAGS+=-static -ffreestanding -nostdlib -fno-exceptions
CFLAGS+=-march=rv64gc -mabi=lp64d

LIB_OUT = kheap_malloc.o

all: $(LIB_OUT)

$(LIB_OUT): my_malloc.c 
	$(CC) $(CFLAGS) -c my_malloc.c -o $@


clean:
	rm $(LIB_OUT)
