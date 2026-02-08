#include "rb_runtime.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <time.h>

/* ── RANDOMIZE seed ──────────────────────────────────── */

void rb_randomize(int32_t seed) {
    srand((unsigned int)seed);
}

/* ── STRING$(n, code) → ptr ──────────────────────────── */

rb_string_t* rb_fn_string_s(int32_t n, int32_t char_code) {
    if (n <= 0) return rb_string_alloc("");
    rb_string_t* result = (rb_string_t*)malloc(sizeof(rb_string_t) + n + 1);
    if (!result) rb_panic("out of memory in STRING$");
    result->refcount = 1;
    result->length = n;
    memset(result->data, (char)(unsigned char)char_code, n);
    result->data[n] = '\0';
    return result;
}

/* ── SPACE$(n) → ptr ─────────────────────────────────── */

rb_string_t* rb_fn_space_s(int32_t n) {
    return rb_fn_string_s(n, 32);
}

/* ── PRINT USING format$, value ──────────────────────── */

void rb_print_using_float(rb_string_t* fmt, float value) {
    if (!fmt) { printf("%g", (double)value); return; }

    /* Count # and . in format to determine width and decimals */
    int32_t total_hashes = 0;
    int32_t decimals = -1;
    int32_t dot_pos = -1;

    for (int32_t i = 0; i < fmt->length; i++) {
        if (fmt->data[i] == '#') total_hashes++;
        if (fmt->data[i] == '.' && dot_pos < 0) dot_pos = i;
    }

    if (dot_pos >= 0) {
        /* Count hashes after the dot */
        decimals = 0;
        for (int32_t i = dot_pos + 1; i < fmt->length; i++) {
            if (fmt->data[i] == '#') decimals++;
            else break;
        }
    }

    int32_t width = total_hashes + (dot_pos >= 0 ? 1 : 0);
    if (decimals >= 0) {
        printf("%*.*f", (int)width, (int)decimals, (double)value);
    } else {
        printf("%*g", (int)width, (double)value);
    }
}

void rb_print_using_int(rb_string_t* fmt, int32_t value) {
    rb_print_using_float(fmt, (float)value);
}

void rb_print_using_string(rb_string_t* fmt, rb_string_t* value) {
    if (!fmt || !value) return;

    /* For strings, the format width is the length of the format pattern */
    int32_t width = fmt->length;
    if (value->length >= width) {
        /* Truncate to width */
        for (int32_t i = 0; i < width; i++) {
            putchar(value->data[i]);
        }
    } else {
        /* Pad with spaces */
        printf("%s", value->data);
        for (int32_t i = value->length; i < width; i++) {
            putchar(' ');
        }
    }
}
