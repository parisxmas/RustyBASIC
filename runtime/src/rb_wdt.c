#include "rb_runtime.h"
#include <stdio.h>

#ifdef ESP_PLATFORM
#include "esp_task_wdt.h"

void rb_wdt_enable(int32_t timeout_ms) {
    esp_task_wdt_config_t cfg = { .timeout_ms = (uint32_t)timeout_ms, .idle_core_mask = 0x03, .trigger_panic = true };
    esp_task_wdt_reconfigure(&cfg);
    esp_task_wdt_add(NULL);
}

void rb_wdt_feed(void) {
    esp_task_wdt_reset();
}

void rb_wdt_disable(void) {
    esp_task_wdt_delete(NULL);
}

#else

void rb_wdt_enable(int32_t timeout_ms) { printf("[WDT] enable %d ms\n", timeout_ms); }
void rb_wdt_feed(void) { printf("[WDT] feed\n"); }
void rb_wdt_disable(void) { printf("[WDT] disable\n"); }

#endif
