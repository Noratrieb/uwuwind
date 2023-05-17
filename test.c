#include<stdio.h>
#define __USE_GNU
#include<dlfcn.h>
#include<elf.h>

int main() {
    printf("%d", PT_GNU_EH_FRAME);
}