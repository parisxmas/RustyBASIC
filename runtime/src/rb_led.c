#include "rb_runtime.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef ESP_PLATFORM
#include "led_strip.h"

static led_strip_handle_t led_strip = NULL;
static int led_count = 0;
#endif

void rb_led_setup(int32_t pin, int32_t count) {
#ifdef ESP_PLATFORM
    if (led_strip) {
        led_strip_del(led_strip);
        led_strip = NULL;
    }
    led_count = count;
    led_strip_config_t strip_config = {
        .strip_gpio_num = pin,
        .max_leds = count,
        .led_pixel_format = LED_PIXEL_FORMAT_GRB,
        .led_model = LED_MODEL_WS2812,
    };
    led_strip_rmt_config_t rmt_config = {
        .resolution_hz = 10 * 1000 * 1000, /* 10 MHz */
        .flags.with_dma = false,
    };
    led_strip_new_rmt_device(&strip_config, &rmt_config, &led_strip);
    led_strip_clear(led_strip);
#else
    printf("[LED] setup: pin=%d, count=%d\n", (int)pin, (int)count);
#endif
}

void rb_led_set(int32_t index, int32_t r, int32_t g, int32_t b) {
#ifdef ESP_PLATFORM
    if (led_strip) {
        led_strip_set_pixel(led_strip, index, r, g, b);
    }
#else
    printf("[LED] set: index=%d, r=%d, g=%d, b=%d\n",
           (int)index, (int)r, (int)g, (int)b);
#endif
}

void rb_led_show(void) {
#ifdef ESP_PLATFORM
    if (led_strip) {
        led_strip_refresh(led_strip);
    }
#else
    printf("[LED] show\n");
#endif
}

void rb_led_clear(void) {
#ifdef ESP_PLATFORM
    if (led_strip) {
        led_strip_clear(led_strip);
        led_strip_refresh(led_strip);
    }
#else
    printf("[LED] clear\n");
#endif
}
