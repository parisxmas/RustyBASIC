#include "rb_runtime.h"
#include <math.h>
#include <stdlib.h>
#include <time.h>

/* ── SQR(x) -> f32 ─────────────────────────────────── */

float rb_fn_sqr(float x) {
    return sqrtf(x);
}

/* ── ABS(x) -> f32 ─────────────────────────────────── */

float rb_fn_abs(float x) {
    return fabsf(x);
}

/* ── SIN(x) -> f32 ─────────────────────────────────── */

float rb_fn_sin(float x) {
    return sinf(x);
}

/* ── COS(x) -> f32 ─────────────────────────────────── */

float rb_fn_cos(float x) {
    return cosf(x);
}

/* ── TAN(x) -> f32 ─────────────────────────────────── */

float rb_fn_tan(float x) {
    return tanf(x);
}

/* ── ATN(x) -> f32 ─────────────────────────────────── */

float rb_fn_atn(float x) {
    return atanf(x);
}

/* ── LOG(x) -> f32 ─────────────────────────────────── */

float rb_fn_log(float x) {
    return logf(x);
}

/* ── EXP(x) -> f32 ─────────────────────────────────── */

float rb_fn_exp(float x) {
    return expf(x);
}

/* ── INT(x) -> i32  (floor toward -infinity) ───────── */

int32_t rb_fn_int(float x) {
    return (int32_t)floorf(x);
}

/* ── FIX(x) -> i32  (truncate toward zero) ─────────── */

int32_t rb_fn_fix(float x) {
    return (int32_t)truncf(x);
}

/* ── SGN(x) -> i32  (-1, 0, or 1) ──────────────────── */

int32_t rb_fn_sgn(float x) {
    if (x > 0.0f) return 1;
    if (x < 0.0f) return -1;
    return 0;
}

/* ── RND -> f32  (random in [0, 1)) ─────────────────── */

float rb_fn_rnd(void) {
    static int seeded = 0;
    if (!seeded) {
        srand((unsigned int)time(NULL));
        seeded = 1;
    }
    return ((float)rand()) / ((float)RAND_MAX + 1.0f);
}
