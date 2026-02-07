#include "rb_runtime.h"
#include <stdio.h>

void rb_print_int(int32_t value) {
    printf("%d", (int)value);
}

void rb_print_float(float value) {
    printf("%g", (double)value);
}

void rb_print_string(rb_string_t* s) {
    if (s) {
        printf("%s", s->data);
    }
}

void rb_print_newline(void) {
    printf("\n");
#ifdef ESP_PLATFORM
    fflush(stdout);
#endif
}
