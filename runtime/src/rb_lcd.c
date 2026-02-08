#include "rb_runtime.h"
#include <stdio.h>

void rb_lcd_init(int32_t cols, int32_t rows) {
#ifdef ESP_PLATFORM
    (void)cols; (void)rows;
#else
    fprintf(stderr, "[stub] LCD.INIT %dx%d\n", (int)cols, (int)rows);
#endif
}

void rb_lcd_print(rb_string_t* text) {
#ifdef ESP_PLATFORM
    (void)text;
#else
    fprintf(stderr, "[stub] LCD.PRINT \"%s\"\n", text ? text->data : "");
#endif
}

void rb_lcd_clear(void) {
#ifdef ESP_PLATFORM
    /* clear LCD */
#else
    fprintf(stderr, "[stub] LCD.CLEAR\n");
#endif
}

void rb_lcd_pos(int32_t col, int32_t row) {
#ifdef ESP_PLATFORM
    (void)col; (void)row;
#else
    fprintf(stderr, "[stub] LCD.POS %d,%d\n", (int)col, (int)row);
#endif
}
