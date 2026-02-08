#include "rb_runtime.h"
#include <stdio.h>

#ifdef ESP_PLATFORM
#include "driver/ledc.h"
#endif

void rb_servo_attach(int32_t channel, int32_t pin) {
#ifdef ESP_PLATFORM
    ledc_timer_config_t timer_conf = {
        .speed_mode = LEDC_LOW_SPEED_MODE,
        .timer_num = LEDC_TIMER_1,
        .duty_resolution = LEDC_TIMER_14_BIT,
        .freq_hz = 50,
        .clk_cfg = LEDC_AUTO_CLK,
    };
    ledc_timer_config(&timer_conf);
    ledc_channel_config_t chan_conf = {
        .speed_mode = LEDC_LOW_SPEED_MODE,
        .channel = (ledc_channel_t)channel,
        .timer_sel = LEDC_TIMER_1,
        .gpio_num = pin,
        .duty = 0,
        .hpoint = 0,
    };
    ledc_channel_config(&chan_conf);
#else
    (void)channel; (void)pin;
    fprintf(stderr, "[stub] SERVO.ATTACH %d, %d\n", (int)channel, (int)pin);
#endif
}

void rb_servo_write(int32_t channel, int32_t angle) {
#ifdef ESP_PLATFORM
    int32_t duty = 819 + (angle * 3277) / 180;
    ledc_set_duty(LEDC_LOW_SPEED_MODE, (ledc_channel_t)channel, duty);
    ledc_update_duty(LEDC_LOW_SPEED_MODE, (ledc_channel_t)channel);
#else
    (void)channel; (void)angle;
    fprintf(stderr, "[stub] SERVO.WRITE %d, %d\n", (int)channel, (int)angle);
#endif
}
