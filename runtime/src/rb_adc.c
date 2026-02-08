#include "rb_runtime.h"
#include <stdio.h>

#ifdef ESP_PLATFORM
#include "esp_adc/adc_oneshot.h"

static adc_oneshot_unit_handle_t adc_handle = NULL;

static void ensure_adc_init(void) {
    if (!adc_handle) {
        adc_oneshot_unit_init_cfg_t cfg = {
            .unit_id = ADC_UNIT_1,
        };
        adc_oneshot_new_unit(&cfg, &adc_handle);
    }
}
#endif

int32_t rb_adc_read(int32_t pin) {
#ifdef ESP_PLATFORM
    ensure_adc_init();
    adc_oneshot_chan_cfg_t chan_cfg = {
        .atten = ADC_ATTEN_DB_12,
        .bitwidth = ADC_BITWIDTH_DEFAULT,
    };
    adc_oneshot_config_channel(adc_handle, (adc_channel_t)pin, &chan_cfg);
    int value = 0;
    adc_oneshot_read(adc_handle, (adc_channel_t)pin, &value);
    return (int32_t)value;
#else
    printf("[ADC] read: pin=%d\n", (int)pin);
    return 0;
#endif
}
