#include "stdlib.h"
#include "stdio.h"
#include "my_malloc.h"
#include "stdint.h"
#include "assert.h"

//#define PRINT_ERRORMSG
#define COALESCING_ON

#define DLL_NULL(li) (li)
#define ROUND_UP(n) (((n) + 7) & (~7))
#define MINIMAL_MEMORY  8


typedef struct memblk{
    struct memblk *prev;
    struct memblk *next;
    usize blk_size;//memory block size(exclude memblk structure)
    void *begin;
} memblk;

struct mem_controller{
    memblk *avail_list;
    memblk *using_list;

    int all_empty;
    void *raw_mem;
} mman;

#define FREE_HEAD  (&(mman.avail_list))
#define USING_HEAD  (&(mman.using_list))

static memblk *find_by_size_memblk(memblk **head, usize _blk_size){
    memblk *pt = *head;
    do{
        if(pt->blk_size >= _blk_size)
            return pt;

        pt = pt->next;
    }while(pt->next != DLL_NULL(pt));

    if(pt->blk_size >= _blk_size)
        return pt;
    
    return NULL;
}

/*
insert_memblk() - insert an memblk struture into ordered list
@head: A pointer to the head pointer of the list
@entry: A pointer to memblk structure

This function will insert the entry into the ordered list and keep the ascending order.
The field for compare is memblk.begin, and the insert is done without deep copy

Note in order to keep the ascending properties of the list, insert_memblk() will reject
the request once head is not the head of the list. BUT insert_memblk() will not free the
entry pointer.

Return: No return value, all error message will be printed on screen
*/
static void insert_memblk(memblk **head, memblk *entry){
    if(head == NULL){ //illegal case
        return;
    }
    if(*head == NULL){ //empty list
        entry->next = entry;
        entry->prev = entry;
        *head = entry;
        return;
    }

    memblk *pt = *head;
    if(pt->prev != DLL_NULL(pt)){
        return;
    }

    if ((*head)->begin > entry->begin){//add before head
        entry->next = *head;
        entry->prev = entry;
        (*head)->prev = entry;
        (*head) = entry;
        return;
    }else{
        while(pt->next->begin < entry->begin){
            pt = pt->next;
            if(pt->next == DLL_NULL(pt)){//add after tail
                pt->next = entry;
                entry->prev = pt;
                entry->next = DLL_NULL(entry);
                return;
            }
        }
        entry->next = pt->next;
        entry->prev = pt;
        pt->next->prev = entry;
        pt->next = entry;
    }
    return;
}

/*
find_by_addr_memblk() - find the entry that match to the addr
@head: A pointer to the head pointer of the list
@addr: The address that find_by_addr_memblk() is going to search

find_by_addr_memblk() perform a search on the whole list for exact match of addr,
the field for compare is memblk.begin

Return: address of first matched block, or NULL for not find

There are some cases, include passing in a null head pointer, that can make
find_by_addr_memblk() fails. The return value is still NULL, but the error message will be printed
on screen
*/
static memblk *find_by_addr_memblk(memblk **head, void *addr){
    if(head == NULL){ //illegal case
        return NULL;
    }
    if(*head == NULL){ //empty list
        return NULL;
    }

    memblk *pt = *head;
    if(pt->prev != DLL_NULL(pt)){ //head is not the head
        return NULL;
    }
    do{
        if(pt->begin == addr){
            return pt;
        }
        pt = pt->next;
    }while(pt->next != DLL_NULL(pt));

    if(pt->begin == addr)
        return pt;
    else
        return NULL;
}

/*
delete_memblk() - this function unlink the record from link list

Note for test purpose, delete_memblk() will also call free() to free 
the memory of the found record, but we need to do it in another way in
malloc lab

@head: A pointer to the head pointer of the list
@addr: The address that delete_memblk() will search and unlink

delete_memblk() conduct a search on the list and unlink the matched entry

Return: Address of unlinked record, NULL if target DNE
*/
static memblk *delete_memblk(memblk **head, void *addr){
    memblk *target = find_by_addr_memblk(head, addr);
    if(target == NULL){
        return NULL;
    }
    if(target->prev == DLL_NULL(target)){//if target is head
        if(target->next == DLL_NULL(target)){//if target is the only one in list
            *head = NULL;
            return target;
        }else{
            *head = (*head)->next;
            target->next->prev = DLL_NULL(target->next);
            return target;
        }
    }else if (target->next == DLL_NULL(target)){//if target is tail
        target->prev->next = DLL_NULL(target->prev);
        return target;
    }else{
        target->prev->next =target->next;
        target->next->prev = target->prev;
        return target;
    }
}


/*
new_memblk() - initialize and insert a memory block structure

@blk_size: size of the requested memory

This function will search and intialize memblk in appropriate place. Details of search policy can be
discussed more in future.

Return: A pointer pointed to the allocated structure
*/
static memblk *new_memblk(usize blk_size){
    usize remain_blk_size = 0;
    memblk *target = find_by_size_memblk(FREE_HEAD, blk_size);//minimal requirement

    if(target == NULL){
        return NULL;
    }

    if(target->blk_size < blk_size + sizeof(memblk) + MINIMAL_MEMORY){// space remainning after split not enough to hold record
        memblk *deleted = delete_memblk(FREE_HEAD, target->begin);
        insert_memblk(USING_HEAD, deleted);
        return target;
    }else{
        delete_memblk(FREE_HEAD, target->begin);
        
        remain_blk_size = target->blk_size - blk_size - sizeof(memblk);
        
        target->blk_size = blk_size;
        target->prev = DLL_NULL(target);
        target->next = DLL_NULL(target);
        target->begin = (void*)((usize)target + sizeof(memblk));
        insert_memblk(USING_HEAD, target);

        memblk *remain = (memblk*)((usize)target->begin + target->blk_size);
        
        remain->blk_size = remain_blk_size;
        remain->prev = DLL_NULL(remain);
        remain->next = DLL_NULL(remain);
        remain->begin = (void*)((usize)remain + sizeof(memblk));
        insert_memblk(FREE_HEAD, remain);
        return target;
    }
    return NULL;
}

void kheap_AllocatorInit(usize begin, usize len){
    mman.avail_list = NULL;
    mman.using_list = NULL;
    mman.raw_mem = (void*)begin;
    memblk *new_blk = (memblk*)mman.raw_mem;
    mman.all_empty = 0;

    new_blk->prev = DLL_NULL(new_blk);
    new_blk->next = DLL_NULL(new_blk);
    new_blk->blk_size = len - sizeof(memblk);
    new_blk->begin = (void*)((usize)new_blk + sizeof(memblk));

    insert_memblk(FREE_HEAD, new_blk);
    return;
}



void *MyMalloc(usize size){
    if(size <= 0) return NULL;
    if(mman.avail_list == NULL) return NULL;
    
    memblk *ret = new_memblk(ROUND_UP(size));
    if (ret == NULL){
        return NULL;
    }else{
        return ret->begin;
    }
}

usize kheap_malloc(usize sz, usize align){
    if ((align < sizeof(char)) || (align & (align - 1)) != 0) {
        return 0;
    }

    void* raw = MyMalloc(sz + align - 1 + sizeof(void*));
    if (!raw) {
        return 0;
    }

    uintptr_t raw_addr = (uintptr_t)raw + sizeof(void*);
    uintptr_t aligned_addr = (raw_addr + (align - 1)) & ~(align - 1);

    ((void**)aligned_addr)[-1] = raw;

    return (usize)aligned_addr;
}

void MyFree(void *buffer){
    if(buffer == NULL){
        return;
    }
    memblk *deleted = delete_memblk(USING_HEAD, buffer);
    if (deleted == NULL) return;
    insert_memblk(FREE_HEAD, deleted);

#ifdef COALESCING_ON
    memblk *prevblk = deleted->prev;
    memblk *nextblk = deleted->next;

    if((usize)deleted->begin + deleted->blk_size == (usize)nextblk){
        deleted->blk_size += sizeof(memblk) + nextblk->blk_size;
        delete_memblk(FREE_HEAD, nextblk->begin);
    }

    if((usize)prevblk->begin + prevblk->blk_size == (usize)deleted){
        prevblk->blk_size += sizeof(memblk) + deleted->blk_size;
        delete_memblk(FREE_HEAD, deleted->begin);
    }
#endif

    return;
}

void kheap_free(void *addr, usize sz, usize align){
    if (addr) {
        // Retrieve the original pointer
        void* raw = ((void**)addr)[-1];
        MyFree(raw);
    }
}

/* static void print_this_blk(memblk *pt){ */
/*     printf("block: 0x%llx\n", (usize)pt); */
/*     printf("\tsize: %lld\n", pt->blk_size); */
/*     printf("\tnext: 0x%llx\n", (usize)((pt->next == DLL_NULL(pt)) ? 0:pt->next)); */
/*     printf("\tprev: 0x%llx\n", (usize)((pt->prev == DLL_NULL(pt)) ? 0:pt->prev)); */
/*     printf("\tbuffer: 0x%llx\n", (usize)pt->begin); */
/*     return; */
/* } */
/* void PrintMyMallocFreeList(){ */
/*     // printf("\n-------------print start-------------\n"); */
/*     memblk *pt = mman.avail_list; */
    
/*     if(pt == NULL){ */
/*         return; */
/*     } */

/*     while(pt->next != DLL_NULL(pt)){ */
/*         if(mman.all_empty){ */
/*             if(pt->next != DLL_NULL(pt)){ */
/*                 assert(pt->begin + pt->blk_size == pt->next); */
/*             } */
/*         } */
/*         print_this_blk(pt); */
/*         pt = pt->next; */
/*     } */
/*     print_this_blk(pt); */
/*     // printf("\n-------------print end-------------\n"); */
/*     return; */
/* } */

/* void* debug(void* args){ */
/*     mman.all_empty = 1; */
/*     return NULL; */
/* } */
