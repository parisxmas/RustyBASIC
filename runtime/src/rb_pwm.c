#include "rb_runtime.h"
#include <stdio.h>

#ifdef ESP_PLATFORM
#include "driver/ledc.h"
#endif

void rb_pwm_setup(int32_t channel, int32_t pin, int32_t freq, int32_t resolution) {
#ifdef ESP_PLATFORM
    ledc_timer_config_t timer_conf = {
        .speed_mode = LEDC_LOW_SPEED_MODE,
        .timer_num = LEDC_TIMER_0,
        .duty_resolution = (ledc_timer_bit_t)resolution,
        .freq_hz = (uint32_t)freq,
        .clk_cfg = LEDC_AUTO_CLK,
    };
    ledc_timer_config(&timer_conf);

    ledc_channel_config_t ch_conf = {
        .speed_mode = LEDC_LOW_SPEED_MODE,
        .channel = (ledc_channel_t)channel,
        .timer_sel = LEDC_TIMER_0,
        .intr_type = LEDC_INTR_DISABLE,
        .gpio_num = pin,
        .duty = 0,
        .hpoint = 0,
    };
    ledc_channel_config(&ch_conf);
#else
    printf("[PWM] setup: ch=%d, pin=%d, freq=%d, res=%d\n",
           (int)channel, (int)pin, (int)freq, (int)resolution);
#endif
}

void rb_pwm_duty(int32_t channel, int32_t duty) {
#ifdef ESP_PLATFORM
    ledc_set_duty(LEDC_LOW_SPEED_MODE, (ledc_channel_t)channel, (uint32_t)duty);
    ledc_update_duty(LEDC_LOW_SPEED_MODE, (ledc_channel_t)channel);
#else
    printf("[PWM] duty: ch=%d, duty=%d\n", (int)channel, (int)duty);
#endif
}
