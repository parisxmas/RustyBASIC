#include "rb_runtime.h"
#include <stdio.h>

/* ESP32-C3 does NOT have a touch sensor peripheral.
   Touch sensors are available on ESP32, ESP32-S2, and ESP32-S3.
   This file provides a stub implementation. */

int32_t rb_touch_read(int32_t pin) {
    (void)pin;
    fprintf(stderr, "[stub] TOUCH.READ not available on ESP32-C3\n");
    return 0;
}
