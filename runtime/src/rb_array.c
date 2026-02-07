#include "rb_runtime.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

void* rb_array_alloc(int32_t element_size, int32_t total_elements) {
    size_t bytes = (size_t)element_size * (size_t)total_elements;
    void* ptr = malloc(bytes);
    if (!ptr) {
        rb_panic("out of memory in rb_array_alloc");
    }
    memset(ptr, 0, bytes);
    return ptr;
}

void rb_array_free(void* ptr) {
    free(ptr);
}

void rb_array_bounds_check(int32_t index, int32_t size) {
    if (index < 0 || index >= size) {
        fprintf(stderr, "Array index out of bounds: index %d, size %d\n", index, size);
        rb_panic("array index out of bounds");
    }
}
