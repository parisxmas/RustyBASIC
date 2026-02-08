#include "rb_runtime.h"
#include <stdio.h>

#ifdef ESP_PLATFORM
#include "driver/ledc.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#endif

void rb_tone(int32_t pin, int32_t freq, int32_t duration_ms) {
#ifdef ESP_PLATFORM
    ledc_timer_config_t timer_conf = {
        .speed_mode = LEDC_LOW_SPEED_MODE,
        .timer_num = LEDC_TIMER_2,
        .duty_resolution = LEDC_TIMER_8_BIT,
        .freq_hz = (uint32_t)freq,
        .clk_cfg = LEDC_AUTO_CLK,
    };
    ledc_timer_config(&timer_conf);
    ledc_channel_config_t chan_conf = {
        .speed_mode = LEDC_LOW_SPEED_MODE,
        .channel = LEDC_CHANNEL_4,
        .timer_sel = LEDC_TIMER_2,
        .gpio_num = pin,
        .duty = 128,
        .hpoint = 0,
    };
    ledc_channel_config(&chan_conf);
    ledc_set_duty(LEDC_LOW_SPEED_MODE, LEDC_CHANNEL_4, 128);
    ledc_update_duty(LEDC_LOW_SPEED_MODE, LEDC_CHANNEL_4);
    vTaskDelay(pdMS_TO_TICKS(duration_ms));
    ledc_set_duty(LEDC_LOW_SPEED_MODE, LEDC_CHANNEL_4, 0);
    ledc_update_duty(LEDC_LOW_SPEED_MODE, LEDC_CHANNEL_4);
#else
    (void)pin;
    fprintf(stderr, "[stub] TONE %d Hz for %d ms on pin %d\n", (int)freq, (int)duration_ms, (int)pin);
#endif
}
