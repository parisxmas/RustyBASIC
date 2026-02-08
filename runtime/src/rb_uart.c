#include "rb_runtime.h"
#include <stdio.h>

#ifdef ESP_PLATFORM
#include "driver/uart.h"
#endif

void rb_uart_setup(int32_t port, int32_t baud, int32_t tx, int32_t rx) {
#ifdef ESP_PLATFORM
    uart_config_t uart_config = {
        .baud_rate = baud,
        .data_bits = UART_DATA_8_BITS,
        .parity = UART_PARITY_DISABLE,
        .stop_bits = UART_STOP_BITS_1,
        .flow_ctrl = UART_HW_FLOWCTRL_DISABLE,
    };
    uart_param_config((uart_port_t)port, &uart_config);
    uart_set_pin((uart_port_t)port, tx, rx, -1, -1);
    uart_driver_install((uart_port_t)port, 256, 0, 0, NULL, 0);
#else
    printf("[UART] setup: port=%d, baud=%d, tx=%d, rx=%d\n",
           (int)port, (int)baud, (int)tx, (int)rx);
#endif
}

void rb_uart_write(int32_t port, int32_t data) {
#ifdef ESP_PLATFORM
    uint8_t byte = (uint8_t)data;
    uart_write_bytes((uart_port_t)port, &byte, 1);
#else
    printf("[UART] write: port=%d, data=%d\n", (int)port, (int)data);
#endif
}

int32_t rb_uart_read(int32_t port) {
#ifdef ESP_PLATFORM
    uint8_t byte = 0;
    int len = uart_read_bytes((uart_port_t)port, &byte, 1, 100 / portTICK_PERIOD_MS);
    if (len > 0) return (int32_t)byte;
    return -1;
#else
    printf("[UART] read: port=%d\n", (int)port);
    return 0;
#endif
}
