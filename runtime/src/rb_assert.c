#include "rb_runtime.h"
#include <stdio.h>
#include <stdlib.h>

void rb_assert_fail(rb_string_t* message, int32_t offset) {
    if (message && message->length > 0) {
        fprintf(stderr, "ASSERT FAILED: %s\n", message->data);
    } else {
        fprintf(stderr, "ASSERT FAILED at offset %d\n", offset);
    }
    abort();
}
