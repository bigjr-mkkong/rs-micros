PREFIX=riscv64-linux-gnu-
CC=$(PREFIX)gcc

CFLAGS=-Wall -Wextra -pedantic -Wextra -O0 -g
CFLAGS+=-static -ffreestanding -nostdlib -fno-exceptions
CFLAGS+=-march=rv64gc -mabi=lp64d

LIB_OUT = kheap_malloc

all: $(LIB_OUT)

kheap_malloc: my_malloc.c 
	$(CC) $(CFLAGS) -c my_malloc.c -o $@


clean:
	rm $(LIB_OUT)
# simpletest1: $(TESTS)/simpletest1.c $(BIN)/my_malloc.o $(LIBDIR)$(LIBS)
# 	$(CC) $(CFLAGS) $^ -o $(BIN)/$@

# simpletest2: $(TESTS)/simpletest2.c $(BIN)/my_malloc.o $(LIBDIR)$(LIBS)
# 	$(CC) $(CFLAGS) $^ -o $(BIN)/$@

# testnocompact: $(TESTS)/testnocompact.c $(BIN)/my_malloc_nocompact.o $(LIBDIR)$(LIBS)
# 	$(CC) $(CFLAGS) $^ -o $(BIN)/$@

# testmymalloc: $(TESTS)/testmymalloc.c $(BIN)/my_malloc.o $(LIBDIR)$(LIBS)
# 	$(CC) $(CFLAGS) $^ -o $(BIN)/$@

# mytest0: $(TESTS)/mytest0.c $(BIN)/my_malloc.o $(LIBDIR)$(LIBS)
# 	$(CC) $(CFLAGS) $^ -o $(BIN)/$@
	
# mytest1: $(TESTS)/mytest1.c $(BIN)/my_malloc.o $(LIBDIR)$(LIBS)
# 	$(CC) $(CFLAGS) $^ -o $(BIN)/$@

# mytest1-cpp: $(TESTS)/mytest1_cpp.cpp $(BIN)/my_malloc.o $(LIBDIR)$(LIBS)
# 	$(CPP) $(CFLAGS) -c $(TESTS)/mytest1_cpp.cpp -o $(BIN)/mytest1-cpp.o
# 	$(CPP) -o $(BIN)/mytest1-cpp $(BIN)/mytest1-cpp.o $(BIN)/my_malloc.o

