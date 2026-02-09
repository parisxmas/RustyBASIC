#include "rb_runtime.h"
#include <stdio.h>

#ifdef ESP_PLATFORM
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

void rb_yield(void) {
    taskYIELD();
}

void rb_await(int32_t ms) {
    vTaskDelay(pdMS_TO_TICKS(ms));
}

#else
#include <unistd.h>

void rb_yield(void) {
    printf("[ASYNC] Yield (stub)\n");
}

void rb_await(int32_t ms) {
    printf("[ASYNC] Await %d ms (stub)\n", ms);
    usleep(ms * 1000);
}

#endif
