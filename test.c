#include <stdio.h>
#define __USE_GNU
#include <dlfcn.h>
#include <elf.h>

void *get_eh_frame(void *addr)
{
    struct dl_find_object out;
    int ret = _dl_find_object(addr, &out);
    if (ret != 0)
    {
        return NULL;
    }

    return out.dlfo_eh_frame;
}