#include "rb_runtime.h"

#ifdef ESP_PLATFORM
#include "esp_log.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

static const char* TAG = "RustyBASIC";

void app_main(void) {
    ESP_LOGI(TAG, "RustyBASIC program starting...");
    basic_program_entry();
    ESP_LOGI(TAG, "RustyBASIC program finished.");
}

#else
/* Host/testing builds */
int main(void) {
    basic_program_entry();
    return 0;
}
#endif
