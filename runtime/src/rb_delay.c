#include "rb_runtime.h"

#ifdef ESP_PLATFORM
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#else
#include <unistd.h>
#endif

void rb_delay(int32_t ms) {
#ifdef ESP_PLATFORM
    vTaskDelay(ms / portTICK_PERIOD_MS);
#else
    usleep((useconds_t)ms * 1000);
#endif
}
