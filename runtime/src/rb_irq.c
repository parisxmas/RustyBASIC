#include "rb_runtime.h"
#include <stdio.h>

#ifdef ESP_PLATFORM
#include "driver/gpio.h"
#endif

void rb_irq_attach(int32_t pin, int32_t mode) {
#ifdef ESP_PLATFORM
    gpio_int_type_t int_type;
    switch (mode) {
        case 1: int_type = GPIO_INTR_POSEDGE; break;
        case 2: int_type = GPIO_INTR_NEGEDGE; break;
        case 3: int_type = GPIO_INTR_ANYEDGE; break;
        default: int_type = GPIO_INTR_POSEDGE; break;
    }
    gpio_set_intr_type((gpio_num_t)pin, int_type);
    gpio_intr_enable((gpio_num_t)pin);
#else
    (void)pin; (void)mode;
    fprintf(stderr, "[stub] IRQ.ATTACH pin=%d mode=%d\n", (int)pin, (int)mode);
#endif
}

void rb_irq_detach(int32_t pin) {
#ifdef ESP_PLATFORM
    gpio_intr_disable((gpio_num_t)pin);
#else
    (void)pin;
    fprintf(stderr, "[stub] IRQ.DETACH pin=%d\n", (int)pin);
#endif
}
