#include "rb_runtime.h"
#include <stdio.h>

#ifdef ESP_PLATFORM
#include "driver/gpio.h"
#endif

void rb_gpio_mode(int32_t pin, int32_t mode) {
#ifdef ESP_PLATFORM
    gpio_config_t io_conf = {
        .pin_bit_mask = (1ULL << pin),
        .mode = (mode == 1) ? GPIO_MODE_OUTPUT : GPIO_MODE_INPUT,
        .pull_up_en = GPIO_PULLUP_DISABLE,
        .pull_down_en = GPIO_PULLDOWN_DISABLE,
        .intr_type = GPIO_INTR_DISABLE,
    };
    gpio_config(&io_conf);
#else
    printf("[GPIO] mode: pin=%d, mode=%d\n", (int)pin, (int)mode);
#endif
}

void rb_gpio_set(int32_t pin, int32_t value) {
#ifdef ESP_PLATFORM
    gpio_set_level((gpio_num_t)pin, value);
#else
    printf("[GPIO] set: pin=%d, value=%d\n", (int)pin, (int)value);
#endif
}

int32_t rb_gpio_read(int32_t pin) {
#ifdef ESP_PLATFORM
    return (int32_t)gpio_get_level((gpio_num_t)pin);
#else
    printf("[GPIO] read: pin=%d\n", (int)pin);
    return 0;
#endif
}
