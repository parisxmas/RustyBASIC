#include "rb_runtime.h"
#include <stdio.h>

#ifdef ESP_PLATFORM
#include "driver/touch_pad.h"
#endif

int32_t rb_touch_read(int32_t pin) {
#ifdef ESP_PLATFORM
    uint32_t val = 0;
    touch_pad_read_raw_data((touch_pad_t)pin, &val);
    return (int32_t)val;
#else
    (void)pin;
    fprintf(stderr, "[stub] TOUCH.READ not supported on host\n");
    return 0;
#endif
}
