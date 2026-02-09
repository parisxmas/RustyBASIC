#include "rb_runtime.h"
#include <stdio.h>
#include <string.h>
#include <time.h>

#define MAX_CRON_JOBS 16

typedef struct {
    int32_t id;
    char expr[64];
    int active;
} cron_job_t;

static cron_job_t jobs[MAX_CRON_JOBS];
static int job_count = 0;

void rb_cron_add(int32_t id, rb_string_t* expr) {
    if (job_count < MAX_CRON_JOBS) {
        jobs[job_count].id = id;
        snprintf(jobs[job_count].expr, sizeof(jobs[job_count].expr), "%.*s", expr->len, expr->data);
        jobs[job_count].active = 1;
        job_count++;
    }
    printf("[CRON] Added job %d: %.*s\n", id, expr->len, expr->data);
}

/* Simple cron check — matches minute-based expressions */
int32_t rb_cron_check(int32_t id) {
    time_t now = time(NULL);
    struct tm *t = localtime(&now);
    for (int i = 0; i < job_count; i++) {
        if (jobs[i].id == id && jobs[i].active) {
            /* Simple: if expr is "*" always fires, otherwise match minute */
            if (jobs[i].expr[0] == '*') return 1;
            int minute = 0;
            if (sscanf(jobs[i].expr, "%d", &minute) == 1) {
                return (t->tm_min == minute) ? 1 : 0;
            }
        }
    }
    return 0;
}

void rb_cron_remove(int32_t id) {
    for (int i = 0; i < job_count; i++) {
        if (jobs[i].id == id) {
            jobs[i].active = 0;
            printf("[CRON] Removed job %d\n", id);
            return;
        }
    }
}
