#include "rb_runtime.h"
#include <stdio.h>
#include <stdlib.h>

#ifdef ESP_PLATFORM
#include "esp_sleep.h"
#endif

void rb_deepsleep(int32_t ms) {
#ifdef ESP_PLATFORM
    uint64_t us = (uint64_t)ms * 1000ULL;
    esp_deep_sleep(us);
    /* does not return */
#else
    printf("[DEEPSLEEP] entering deep sleep for %d ms\n", (int)ms);
    exit(0);
#endif
}
