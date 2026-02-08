#include "rb_runtime.h"
#include <stdio.h>

void rb_oled_init(int32_t width, int32_t height) {
#ifdef ESP_PLATFORM
    /* SSD1306 I2C initialization would go here */
    (void)width; (void)height;
#else
    fprintf(stderr, "[stub] OLED.INIT %dx%d\n", (int)width, (int)height);
#endif
}

void rb_oled_print(int32_t x, int32_t y, rb_string_t* text) {
#ifdef ESP_PLATFORM
    (void)x; (void)y; (void)text;
#else
    fprintf(stderr, "[stub] OLED.PRINT %d,%d \"%s\"\n", (int)x, (int)y, text ? text->data : "");
#endif
}

void rb_oled_pixel(int32_t x, int32_t y, int32_t color) {
#ifdef ESP_PLATFORM
    (void)x; (void)y; (void)color;
#else
    fprintf(stderr, "[stub] OLED.PIXEL %d,%d color=%d\n", (int)x, (int)y, (int)color);
#endif
}

void rb_oled_line(int32_t x1, int32_t y1, int32_t x2, int32_t y2, int32_t color) {
#ifdef ESP_PLATFORM
    (void)x1; (void)y1; (void)x2; (void)y2; (void)color;
#else
    fprintf(stderr, "[stub] OLED.LINE (%d,%d)-(%d,%d) color=%d\n", (int)x1, (int)y1, (int)x2, (int)y2, (int)color);
#endif
}

void rb_oled_clear(void) {
#ifdef ESP_PLATFORM
    /* clear OLED framebuffer */
#else
    fprintf(stderr, "[stub] OLED.CLEAR\n");
#endif
}

void rb_oled_show(void) {
#ifdef ESP_PLATFORM
    /* flush framebuffer to display */
#else
    fprintf(stderr, "[stub] OLED.SHOW\n");
#endif
}
