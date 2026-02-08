#include "rb_runtime.h"
#include <stdio.h>
#include <stdlib.h>
#include <setjmp.h>
#include <string.h>

#define MAX_TRY_DEPTH 16

static jmp_buf try_stack[MAX_TRY_DEPTH];
static int try_depth = 0;
static char error_message[256] = {0};

int32_t rb_try_begin(void) {
    if (try_depth >= MAX_TRY_DEPTH) {
        fprintf(stderr, "TRY/CATCH nested too deep\n");
        abort();
    }
    int result = setjmp(try_stack[try_depth]);
    if (result == 0) {
        try_depth++;
        return 0;  /* entering try block */
    }
    return 1;  /* caught an error */
}

void rb_try_end(void) {
    if (try_depth > 0) {
        try_depth--;
    }
}

void rb_throw(rb_string_t* message) {
    if (try_depth > 0) {
        try_depth--;
        if (message && message->length > 0) {
            strncpy(error_message, message->data, sizeof(error_message) - 1);
            error_message[sizeof(error_message) - 1] = '\0';
        } else {
            strcpy(error_message, "Unknown error");
        }
        longjmp(try_stack[try_depth], 1);
    } else {
        /* No try block active, treat as fatal */
        if (message && message->length > 0) {
            fprintf(stderr, "Unhandled error: %s\n", message->data);
        }
        abort();
    }
}

rb_string_t* rb_get_error_message(void) {
    return rb_string_alloc(error_message);
}
