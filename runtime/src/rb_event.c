#include "rb_runtime.h"
#include <stdio.h>

#ifdef ESP_PLATFORM
#include "driver/gpio.h"
#include "esp_timer.h"

static void (*gpio_handlers[40])(void) = {0};

static void IRAM_ATTR gpio_isr_handler(void* arg) {
    int pin = (int)(intptr_t)arg;
    if (pin >= 0 && pin < 40 && gpio_handlers[pin]) {
        gpio_handlers[pin]();
    }
}

void rb_on_gpio_change(int32_t pin, void (*handler)(void)) {
    if (pin >= 0 && pin < 40) {
        gpio_handlers[pin] = handler;
        gpio_install_isr_service(0);
        gpio_isr_handler_add((gpio_num_t)pin, gpio_isr_handler, (void*)(intptr_t)pin);
        gpio_set_intr_type((gpio_num_t)pin, GPIO_INTR_ANYEDGE);
    }
}

static void timer_callback(void* arg) {
    void (*handler)(void) = (void (*)(void))arg;
    if (handler) handler();
}

void rb_on_timer(int32_t interval_ms, void (*handler)(void)) {
    esp_timer_create_args_t timer_args = {
        .callback = timer_callback,
        .arg = (void*)handler,
        .name = "rb_timer"
    };
    esp_timer_handle_t timer;
    esp_timer_create(&timer_args, &timer);
    esp_timer_start_periodic(timer, (uint64_t)interval_ms * 1000);
}

void rb_on_mqtt_message(void (*handler)(void)) {
    /* Registered in MQTT event loop — stub for now */
    (void)handler;
    printf("[EVENT] MQTT message handler registered\n");
}

#else

void rb_on_gpio_change(int32_t pin, void (*handler)(void)) {
    (void)pin;
    (void)handler;
    printf("[HOST STUB] ON GPIO.CHANGE %d registered\n", pin);
}

void rb_on_timer(int32_t interval_ms, void (*handler)(void)) {
    (void)interval_ms;
    (void)handler;
    printf("[HOST STUB] ON TIMER %d ms registered\n", interval_ms);
}

void rb_on_mqtt_message(void (*handler)(void)) {
    (void)handler;
    printf("[HOST STUB] ON MQTT.MESSAGE registered\n");
}

#endif
