#include "rb_runtime.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <limits.h>

void rb_array_check_dim_size(int32_t dim_value, int32_t dim_index) {
    if (dim_value < 0) {
        fprintf(stderr, "Negative array dimension size: DIM dimension %d = %d\n",
                dim_index, dim_value);
        rb_panic("negative array dimension size");
    }
}

void* rb_array_alloc(int32_t element_size, int32_t total_elements) {
    if (total_elements <= 0) {
        fprintf(stderr, "Invalid array total size: %d\n", total_elements);
        rb_panic("invalid array size");
    }
    if (element_size <= 0) {
        rb_panic("invalid array element size");
    }
    /* Overflow check: element_size * total_elements must fit in size_t */
    if ((size_t)total_elements > SIZE_MAX / (size_t)element_size) {
        fprintf(stderr, "Array allocation overflow: %d * %d elements\n",
                element_size, total_elements);
        rb_panic("array allocation size overflow");
    }
    size_t bytes = (size_t)element_size * (size_t)total_elements;
    void* ptr = malloc(bytes);
    if (!ptr) {
        fprintf(stderr, "Out of memory allocating %zu bytes for array\n", bytes);
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
