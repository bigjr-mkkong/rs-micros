#if !defined(MY_MALLOC_H)
#define MY_MALLOC_H

#define MAX_MALLOC_SIZE (1024*1024*16)

//#define MAX_MALLOC_SIZE (1024*10)

typedef unsigned long long usize;
void InitMyMalloc();
void *MyMalloc(usize size);
void MyFree(void *buffer);
void PrintMyMallocFreeList();


#endif

