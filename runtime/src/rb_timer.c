#include "rb_runtime.h"
#include <stdio.h>

#ifdef ESP_PLATFORM
#include "esp_timer.h"
static int64_t timer_start_us = 0;
#else
#include <sys/time.h>
static long long timer_start_us = 0;

static long long get_time_us(void) {
    struct timeval tv;
    gettimeofday(&tv, NULL);
    return (long long)tv.tv_sec * 1000000LL + (long long)tv.tv_usec;
}
#endif

void rb_timer_start(void) {
#ifdef ESP_PLATFORM
    timer_start_us = esp_timer_get_time();
#else
    timer_start_us = get_time_us();
#endif
}

int32_t rb_timer_elapsed(void) {
#ifdef ESP_PLATFORM
    int64_t now = esp_timer_get_time();
    return (int32_t)((now - timer_start_us) / 1000);
#else
    long long now = get_time_us();
    return (int32_t)((now - timer_start_us) / 1000);
#endif
}
