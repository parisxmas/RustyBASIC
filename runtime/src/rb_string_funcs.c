#include "rb_runtime.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <ctype.h>

/* ── LEN(s$) → i32 ──────────────────────────────────── */

int32_t rb_fn_len(rb_string_t* s) {
    if (!s) return 0;
    return s->length;
}

/* ── ASC(s$) → i32 ──────────────────────────────────── */

int32_t rb_fn_asc(rb_string_t* s) {
    if (!s || s->length == 0) return 0;
    return (int32_t)(unsigned char)s->data[0];
}

/* ── CHR$(n) → ptr ──────────────────────────────────── */

rb_string_t* rb_fn_chr_s(int32_t code) {
    char buf[2];
    buf[0] = (char)(unsigned char)code;
    buf[1] = '\0';
    return rb_string_alloc(buf);
}

/* ── LEFT$(s$, n) → ptr ─────────────────────────────── */

rb_string_t* rb_fn_left_s(rb_string_t* s, int32_t n) {
    if (!s || n <= 0) return rb_string_alloc("");
    if (n > s->length) n = s->length;

    rb_string_t* result = (rb_string_t*)malloc(sizeof(rb_string_t) + n + 1);
    if (!result) rb_panic("out of memory in rb_fn_left_s");
    result->refcount = 1;
    result->length = n;
    memcpy(result->data, s->data, n);
    result->data[n] = '\0';
    return result;
}

/* ── RIGHT$(s$, n) → ptr ────────────────────────────── */

rb_string_t* rb_fn_right_s(rb_string_t* s, int32_t n) {
    if (!s || n <= 0) return rb_string_alloc("");
    if (n > s->length) n = s->length;
    int32_t start = s->length - n;

    rb_string_t* result = (rb_string_t*)malloc(sizeof(rb_string_t) + n + 1);
    if (!result) rb_panic("out of memory in rb_fn_right_s");
    result->refcount = 1;
    result->length = n;
    memcpy(result->data, s->data + start, n);
    result->data[n] = '\0';
    return result;
}

/* ── MID$(s$, start, len) → ptr  (1-based start) ──── */

rb_string_t* rb_fn_mid_s(rb_string_t* s, int32_t start, int32_t len) {
    if (!s || start < 1 || len <= 0) return rb_string_alloc("");
    int32_t idx = start - 1;  /* convert to 0-based */
    if (idx >= s->length) return rb_string_alloc("");
    if (idx + len > s->length) len = s->length - idx;

    rb_string_t* result = (rb_string_t*)malloc(sizeof(rb_string_t) + len + 1);
    if (!result) rb_panic("out of memory in rb_fn_mid_s");
    result->refcount = 1;
    result->length = len;
    memcpy(result->data, s->data + idx, len);
    result->data[len] = '\0';
    return result;
}

/* ── INSTR(s$, find$) → i32  (1-based, 0 if not found) */

int32_t rb_fn_instr(rb_string_t* s, rb_string_t* find) {
    if (!s || !find) return 0;
    if (find->length == 0) return 1;
    if (s->length == 0) return 0;

    const char* p = strstr(s->data, find->data);
    if (!p) return 0;
    return (int32_t)(p - s->data) + 1;
}

/* ── STR$(n) → ptr ──────────────────────────────────── */

rb_string_t* rb_fn_str_s(float value) {
    char buf[64];
    snprintf(buf, sizeof(buf), "%g", (double)value);
    return rb_string_alloc(buf);
}

/* ── VAL(s$) → f32 ──────────────────────────────────── */

float rb_fn_val(rb_string_t* s) {
    if (!s || s->length == 0) return 0.0f;
    return (float)atof(s->data);
}

/* ── UCASE$(s$) → ptr ───────────────────────────────── */

rb_string_t* rb_fn_ucase_s(rb_string_t* s) {
    if (!s) return rb_string_alloc("");
    rb_string_t* result = (rb_string_t*)malloc(sizeof(rb_string_t) + s->length + 1);
    if (!result) rb_panic("out of memory in rb_fn_ucase_s");
    result->refcount = 1;
    result->length = s->length;
    for (int32_t i = 0; i < s->length; i++) {
        result->data[i] = (char)toupper((unsigned char)s->data[i]);
    }
    result->data[s->length] = '\0';
    return result;
}

/* ── LCASE$(s$) → ptr ───────────────────────────────── */

rb_string_t* rb_fn_lcase_s(rb_string_t* s) {
    if (!s) return rb_string_alloc("");
    rb_string_t* result = (rb_string_t*)malloc(sizeof(rb_string_t) + s->length + 1);
    if (!result) rb_panic("out of memory in rb_fn_lcase_s");
    result->refcount = 1;
    result->length = s->length;
    for (int32_t i = 0; i < s->length; i++) {
        result->data[i] = (char)tolower((unsigned char)s->data[i]);
    }
    result->data[s->length] = '\0';
    return result;
}

/* ── TRIM$(s$) → ptr ────────────────────────────────── */

rb_string_t* rb_fn_trim_s(rb_string_t* s) {
    if (!s || s->length == 0) return rb_string_alloc("");

    const char* start = s->data;
    const char* end = s->data + s->length - 1;

    while (start <= end && isspace((unsigned char)*start)) start++;
    while (end > start && isspace((unsigned char)*end)) end--;

    int32_t len = (int32_t)(end - start + 1);
    if (len <= 0) return rb_string_alloc("");

    rb_string_t* result = (rb_string_t*)malloc(sizeof(rb_string_t) + len + 1);
    if (!result) rb_panic("out of memory in rb_fn_trim_s");
    result->refcount = 1;
    result->length = len;
    memcpy(result->data, start, len);
    result->data[len] = '\0';
    return result;
}
