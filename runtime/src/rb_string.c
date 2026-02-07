#include "rb_runtime.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

rb_string_t* rb_string_alloc(const char* cstr) {
    if (!cstr) return NULL;
    size_t len = strlen(cstr);
    rb_string_t* s = (rb_string_t*)malloc(sizeof(rb_string_t) + len + 1);
    if (!s) {
        rb_panic("out of memory in rb_string_alloc");
    }
    s->refcount = 1;
    s->length = (int32_t)len;
    memcpy(s->data, cstr, len + 1);
    return s;
}

rb_string_t* rb_string_concat(rb_string_t* a, rb_string_t* b) {
    const char* a_data = a ? a->data : "";
    const char* b_data = b ? b->data : "";
    int32_t a_len = a ? a->length : 0;
    int32_t b_len = b ? b->length : 0;
    int32_t total = a_len + b_len;

    rb_string_t* result = (rb_string_t*)malloc(sizeof(rb_string_t) + total + 1);
    if (!result) {
        rb_panic("out of memory in rb_string_concat");
    }
    result->refcount = 1;
    result->length = total;
    memcpy(result->data, a_data, a_len);
    memcpy(result->data + a_len, b_data, b_len);
    result->data[total] = '\0';
    return result;
}

int32_t rb_string_compare(rb_string_t* a, rb_string_t* b) {
    const char* a_data = a ? a->data : "";
    const char* b_data = b ? b->data : "";
    return (int32_t)strcmp(a_data, b_data);
}

void rb_string_retain(rb_string_t* s) {
    if (s) {
        s->refcount++;
    }
}

void rb_string_release(rb_string_t* s) {
    if (s) {
        s->refcount--;
        if (s->refcount <= 0) {
            free(s);
        }
    }
}
