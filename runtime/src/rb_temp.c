#include "rb_runtime.h"
#include <stdio.h>

#ifdef ESP_PLATFORM
#include "driver/temperature_sensor.h"
static temperature_sensor_handle_t temp_handle = NULL;
#endif

float rb_temp_read(void) {
#ifdef ESP_PLATFORM
    if (!temp_handle) {
        temperature_sensor_config_t conf = TEMPERATURE_SENSOR_CONFIG_DEFAULT(-10, 80);
        temperature_sensor_install(&conf, &temp_handle);
        temperature_sensor_enable(temp_handle);
    }
    float result = 0.0f;
    temperature_sensor_get_celsius(temp_handle, &result);
    return result;
#else
    fprintf(stderr, "[stub] TEMP.READ not supported on host\n");
    return 25.0f;
#endif
}
