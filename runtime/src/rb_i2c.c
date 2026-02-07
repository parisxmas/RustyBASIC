#include "rb_runtime.h"
#include <stdio.h>

#ifdef ESP_PLATFORM
#include "driver/i2c.h"
#endif

void rb_i2c_setup(int32_t bus, int32_t sda, int32_t scl, int32_t freq) {
#ifdef ESP_PLATFORM
    i2c_config_t conf = {
        .mode = I2C_MODE_MASTER,
        .sda_io_num = sda,
        .scl_io_num = scl,
        .sda_pullup_en = GPIO_PULLUP_ENABLE,
        .scl_pullup_en = GPIO_PULLUP_ENABLE,
        .master.clk_speed = freq,
    };
    i2c_param_config((i2c_port_t)bus, &conf);
    i2c_driver_install((i2c_port_t)bus, conf.mode, 0, 0, 0);
#else
    printf("[I2C] setup: bus=%d, sda=%d, scl=%d, freq=%d\n",
           (int)bus, (int)sda, (int)scl, (int)freq);
#endif
}

void rb_i2c_write(int32_t addr, int32_t data) {
#ifdef ESP_PLATFORM
    uint8_t buf[1] = { (uint8_t)data };
    i2c_master_write_to_device(I2C_NUM_0, (uint8_t)addr, buf, 1,
                                1000 / portTICK_PERIOD_MS);
#else
    printf("[I2C] write: addr=0x%02x, data=0x%02x\n", (int)addr, (int)data);
#endif
}

int32_t rb_i2c_read(int32_t addr, int32_t length) {
#ifdef ESP_PLATFORM
    uint8_t buf[1] = {0};
    i2c_master_read_from_device(I2C_NUM_0, (uint8_t)addr, buf,
                                 (length > 0) ? 1 : 0,
                                 1000 / portTICK_PERIOD_MS);
    return (int32_t)buf[0];
#else
    printf("[I2C] read: addr=0x%02x, length=%d\n", (int)addr, (int)length);
    return 0;
#endif
}
