#include "rb_runtime.h"
#include <stdio.h>
#include <stdlib.h>

#ifdef ESP_PLATFORM
#include "esp_log.h"
#include "esp_system.h"
#endif

void rb_panic(const char* message) {
#ifdef ESP_PLATFORM
    ESP_LOGE("RustyBASIC", "RUNTIME ERROR: %s", message);
    esp_restart();
#else
    fprintf(stderr, "RUNTIME ERROR: %s\n", message);
    exit(1);
#endif
    /* unreachable, but satisfy noreturn */
    while(1) {}
}
