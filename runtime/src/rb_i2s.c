#include "rb_runtime.h"
#include <stdio.h>
#include <string.h>

#ifdef ESP_PLATFORM
#include "driver/i2s_std.h"

static i2s_chan_handle_t i2s_tx_handle = NULL;

void rb_i2s_init(int32_t rate, int32_t bits, int32_t channels) {
    i2s_chan_config_t chan_cfg = I2S_CHANNEL_DEFAULT_CONFIG(I2S_NUM_0, I2S_ROLE_MASTER);
    i2s_new_channel(&chan_cfg, &i2s_tx_handle, NULL);
    i2s_std_config_t std_cfg = {
        .clk_cfg = I2S_STD_CLK_DEFAULT_CONFIG(rate),
        .slot_cfg = I2S_STD_PHILIPS_SLOT_DEFAULT_CONFIG(bits, channels == 1 ? I2S_SLOT_MODE_MONO : I2S_SLOT_MODE_STEREO),
        .gpio_cfg = { .mclk = I2S_GPIO_UNUSED, .bclk = GPIO_NUM_26, .ws = GPIO_NUM_25, .dout = GPIO_NUM_22, .din = I2S_GPIO_UNUSED },
    };
    i2s_channel_init_std_mode(i2s_tx_handle, &std_cfg);
    i2s_channel_enable(i2s_tx_handle);
}

void rb_i2s_write(rb_string_t* data) {
    if (i2s_tx_handle) {
        size_t written = 0;
        i2s_channel_write(i2s_tx_handle, data->data, data->length, &written, portMAX_DELAY);
    }
}

void rb_i2s_stop(void) {
    if (i2s_tx_handle) {
        i2s_channel_disable(i2s_tx_handle);
        i2s_del_channel(i2s_tx_handle);
        i2s_tx_handle = NULL;
    }
}

#else

void rb_i2s_init(int32_t rate, int32_t bits, int32_t channels) {
    printf("[I2S] init rate=%d bits=%d channels=%d\n", rate, bits, channels);
}

void rb_i2s_write(rb_string_t* data) {
    printf("[I2S] write %d bytes\n", data->length);
}

void rb_i2s_stop(void) {
    printf("[I2S] stop\n");
}

#endif
