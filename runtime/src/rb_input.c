#include "rb_runtime.h"
#include <stdio.h>
#include <string.h>

int32_t rb_input_int(const char* prompt) {
    if (prompt) {
        printf("%s", prompt);
    } else {
        printf("? ");
    }
    fflush(stdout);
    int32_t value = 0;
    scanf("%d", &value);
    return value;
}

float rb_input_float(const char* prompt) {
    if (prompt) {
        printf("%s", prompt);
    } else {
        printf("? ");
    }
    fflush(stdout);
    float value = 0.0f;
    scanf("%f", &value);
    return value;
}

rb_string_t* rb_input_string(const char* prompt) {
    if (prompt) {
        printf("%s", prompt);
    } else {
        printf("? ");
    }
    fflush(stdout);
    char buf[256];
    if (fgets(buf, sizeof(buf), stdin)) {
        /* Strip trailing newline */
        size_t len = strlen(buf);
        if (len > 0 && buf[len - 1] == '\n') {
            buf[len - 1] = '\0';
        }
        return rb_string_alloc(buf);
    }
    return rb_string_alloc("");
}
