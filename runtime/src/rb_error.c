#include "rb_runtime.h"
#include <setjmp.h>
#include <string.h>

jmp_buf rb_error_jmpbuf;
int32_t rb_error_handler_active = 0;

void rb_error_clear(void) {
    rb_error_handler_active = 0;
}
