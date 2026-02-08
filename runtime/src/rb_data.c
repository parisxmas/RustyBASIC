#include "rb_runtime.h"
#include <stdio.h>

extern const int32_t rb_data_types[];
extern const int32_t rb_data_ints[];
extern const float rb_data_floats[];
extern const char* rb_data_strings[];
extern const int32_t rb_data_count;

static int32_t rb_data_index = 0;

int32_t rb_data_read_int(void) {
    if (rb_data_index >= rb_data_count)
        rb_panic("Out of DATA");
    int32_t i = rb_data_index++;
    switch (rb_data_types[i]) {
        case 0: return rb_data_ints[i];
        case 1: return (int32_t)rb_data_floats[i];
        default: rb_panic("Type mismatch in READ: expected number, got string");
    }
}

float rb_data_read_float(void) {
    if (rb_data_index >= rb_data_count)
        rb_panic("Out of DATA");
    int32_t i = rb_data_index++;
    switch (rb_data_types[i]) {
        case 0: return (float)rb_data_ints[i];
        case 1: return rb_data_floats[i];
        default: rb_panic("Type mismatch in READ: expected number, got string");
    }
}

rb_string_t* rb_data_read_string(void) {
    if (rb_data_index >= rb_data_count)
        rb_panic("Out of DATA");
    int32_t i = rb_data_index++;
    switch (rb_data_types[i]) {
        case 0: return rb_fn_str_s((float)rb_data_ints[i]);
        case 1: return rb_fn_str_s(rb_data_floats[i]);
        default: return rb_string_alloc(rb_data_strings[i]);
    }
}

void rb_data_restore(void) {
    rb_data_index = 0;
}
