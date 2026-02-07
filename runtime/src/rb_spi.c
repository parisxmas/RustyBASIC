#include "rb_runtime.h"
#include <stdio.h>

#ifdef ESP_PLATFORM
#include "driver/spi_master.h"
static spi_device_handle_t spi_handle = NULL;
#endif

void rb_spi_setup(int32_t bus, int32_t clk, int32_t mosi, int32_t miso, int32_t freq) {
#ifdef ESP_PLATFORM
    spi_bus_config_t buscfg = {
        .miso_io_num = miso,
        .mosi_io_num = mosi,
        .sclk_io_num = clk,
        .quadwp_io_num = -1,
        .quadhd_io_num = -1,
    };
    spi_bus_initialize((spi_host_device_t)bus, &buscfg, SPI_DMA_CH_AUTO);

    spi_device_interface_config_t devcfg = {
        .clock_speed_hz = freq,
        .mode = 0,
        .spics_io_num = -1,
        .queue_size = 1,
    };
    spi_bus_add_device((spi_host_device_t)bus, &devcfg, &spi_handle);
#else
    printf("[SPI] setup: bus=%d, clk=%d, mosi=%d, miso=%d, freq=%d\n",
           (int)bus, (int)clk, (int)mosi, (int)miso, (int)freq);
#endif
}

int32_t rb_spi_transfer(int32_t data) {
#ifdef ESP_PLATFORM
    uint8_t tx = (uint8_t)data;
    uint8_t rx = 0;
    spi_transaction_t t = {
        .length = 8,
        .tx_buffer = &tx,
        .rx_buffer = &rx,
    };
    if (spi_handle) {
        spi_device_transmit(spi_handle, &t);
    }
    return (int32_t)rx;
#else
    printf("[SPI] transfer: data=0x%02x\n", (int)data);
    return 0;
#endif
}
