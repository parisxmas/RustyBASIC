#ifndef RB_RUNTIME_H
#define RB_RUNTIME_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ── String type (refcounted) ─────────────────────────── */

typedef struct rb_string {
    int32_t refcount;
    int32_t length;
    char data[];  /* flexible array member */
} rb_string_t;

rb_string_t* rb_string_alloc(const char* cstr);
rb_string_t* rb_string_concat(rb_string_t* a, rb_string_t* b);
int32_t rb_string_compare(rb_string_t* a, rb_string_t* b);
void rb_string_retain(rb_string_t* s);
void rb_string_release(rb_string_t* s);

/* ── Print ────────────────────────────────────────────── */

void rb_print_int(int32_t value);
void rb_print_float(float value);
void rb_print_string(rb_string_t* s);
void rb_print_newline(void);

/* ── Input ────────────────────────────────────────────── */

int32_t rb_input_int(const char* prompt);
float rb_input_float(const char* prompt);
rb_string_t* rb_input_string(const char* prompt);

/* ── Panic ────────────────────────────────────────────── */

void rb_panic(const char* message) __attribute__((noreturn));

/* ── GPIO ─────────────────────────────────────────────── */

void rb_gpio_mode(int32_t pin, int32_t mode);
void rb_gpio_set(int32_t pin, int32_t value);
int32_t rb_gpio_read(int32_t pin);

/* ── Delay ────────────────────────────────────────────── */

void rb_delay(int32_t ms);

/* ── I2C ──────────────────────────────────────────────── */

void rb_i2c_setup(int32_t bus, int32_t sda, int32_t scl, int32_t freq);
void rb_i2c_write(int32_t addr, int32_t data);
int32_t rb_i2c_read(int32_t addr, int32_t length);

/* ── SPI ──────────────────────────────────────────────── */

void rb_spi_setup(int32_t bus, int32_t clk, int32_t mosi, int32_t miso, int32_t freq);
int32_t rb_spi_transfer(int32_t data);

/* ── WiFi ─────────────────────────────────────────────── */

void rb_wifi_connect(rb_string_t* ssid, rb_string_t* password);
int32_t rb_wifi_status(void);
void rb_wifi_disconnect(void);

/* ── ADC ──────────────────────────────────────────────── */

int32_t rb_adc_read(int32_t pin);

/* ── PWM ──────────────────────────────────────────────── */

void rb_pwm_setup(int32_t channel, int32_t pin, int32_t freq, int32_t resolution);
void rb_pwm_duty(int32_t channel, int32_t duty);

/* ── UART ─────────────────────────────────────────────── */

void rb_uart_setup(int32_t port, int32_t baud, int32_t tx, int32_t rx);
void rb_uart_write(int32_t port, int32_t data);
int32_t rb_uart_read(int32_t port);

/* ── Timer ────────────────────────────────────────────── */

void rb_timer_start(void);
int32_t rb_timer_elapsed(void);

/* ── HTTP ─────────────────────────────────────────────── */

rb_string_t* rb_http_get(rb_string_t* url);
rb_string_t* rb_http_post(rb_string_t* url, rb_string_t* body);

/* ── NVS (Non-Volatile Storage) ───────────────────────── */

void rb_nvs_write(rb_string_t* key, int32_t value);
int32_t rb_nvs_read(rb_string_t* key);

/* ── MQTT ─────────────────────────────────────────────── */

void rb_mqtt_connect(rb_string_t* broker, int32_t port);
void rb_mqtt_disconnect(void);
void rb_mqtt_publish(rb_string_t* topic, rb_string_t* message);
void rb_mqtt_subscribe(rb_string_t* topic);
rb_string_t* rb_mqtt_receive(void);

/* ── BLE ──────────────────────────────────────────────── */

void rb_ble_init(rb_string_t* name);
void rb_ble_advertise(int32_t mode);
rb_string_t* rb_ble_scan(void);
void rb_ble_send(rb_string_t* data);
rb_string_t* rb_ble_receive(void);

/* ── JSON ─────────────────────────────────────────────── */

rb_string_t* rb_json_get(rb_string_t* json, rb_string_t* key);
rb_string_t* rb_json_set(rb_string_t* json, rb_string_t* key, rb_string_t* value);
int32_t rb_json_count(rb_string_t* json);

/* ── NeoPixel (WS2812) ────────────────────────────────── */

void rb_led_setup(int32_t pin, int32_t count);
void rb_led_set(int32_t index, int32_t r, int32_t g, int32_t b);
void rb_led_show(void);
void rb_led_clear(void);

/* ── Deep Sleep ───────────────────────────────────────── */

void rb_deepsleep(int32_t ms);

/* ── ESP-NOW ──────────────────────────────────────────── */

void rb_espnow_init(void);
void rb_espnow_send(rb_string_t* peer, rb_string_t* data);
rb_string_t* rb_espnow_receive(void);

/* ── DATA/READ/RESTORE ───────────────────────────────── */

int32_t rb_data_read_int(void);
float rb_data_read_float(void);
rb_string_t* rb_data_read_string(void);
void rb_data_restore(void);

/* ── Classic BASIC extensions ────────────────────────── */

void rb_randomize(int32_t seed);
void rb_print_using_int(rb_string_t* fmt, int32_t value);
void rb_print_using_float(rb_string_t* fmt, float value);
void rb_print_using_string(rb_string_t* fmt, rb_string_t* value);
void rb_error_clear(void);
rb_string_t* rb_fn_string_s(int32_t n, int32_t char_code);
rb_string_t* rb_fn_space_s(int32_t n);

/* ── Touch sensor ────────────────────────────────────── */

int32_t rb_touch_read(int32_t pin);

/* ── Servo ───────────────────────────────────────────── */

void rb_servo_attach(int32_t channel, int32_t pin);
void rb_servo_write(int32_t channel, int32_t angle);

/* ── Tone ────────────────────────────────────────────── */

void rb_tone(int32_t pin, int32_t freq, int32_t duration_ms);

/* ── IRQ ─────────────────────────────────────────────── */

void rb_irq_attach(int32_t pin, int32_t mode);
void rb_irq_detach(int32_t pin);

/* ── Temperature sensor ──────────────────────────────── */

float rb_temp_read(void);

/* ── OTA ─────────────────────────────────────────────── */

void rb_ota_update(rb_string_t* url);

/* ── OLED display ────────────────────────────────────── */

void rb_oled_init(int32_t width, int32_t height);
void rb_oled_print(int32_t x, int32_t y, rb_string_t* text);
void rb_oled_pixel(int32_t x, int32_t y, int32_t color);
void rb_oled_line(int32_t x1, int32_t y1, int32_t x2, int32_t y2, int32_t color);
void rb_oled_clear(void);
void rb_oled_show(void);

/* ── LCD display ─────────────────────────────────────── */

void rb_lcd_init(int32_t cols, int32_t rows);
void rb_lcd_print(rb_string_t* text);
void rb_lcd_clear(void);
void rb_lcd_pos(int32_t col, int32_t row);

/* ── UDP ─────────────────────────────────────────────── */

void rb_udp_init(int32_t port);
void rb_udp_send(rb_string_t* host, int32_t port, rb_string_t* data);
rb_string_t* rb_udp_receive(void);

/* ── Arrays ───────────────────────────────────────────── */

void rb_array_check_dim_size(int32_t dim_value, int32_t dim_index);
void* rb_array_alloc(int32_t element_size, int32_t total_elements);
void rb_array_free(void* ptr);
void rb_array_bounds_check(int32_t index, int32_t size);

/* ── String built-ins ─────────────────────────────────── */

int32_t rb_fn_len(rb_string_t* s);
int32_t rb_fn_asc(rb_string_t* s);
rb_string_t* rb_fn_chr_s(int32_t code);
rb_string_t* rb_fn_left_s(rb_string_t* s, int32_t n);
rb_string_t* rb_fn_right_s(rb_string_t* s, int32_t n);
rb_string_t* rb_fn_mid_s(rb_string_t* s, int32_t start, int32_t len);
int32_t rb_fn_instr(rb_string_t* s, rb_string_t* find);
rb_string_t* rb_fn_str_s(float value);
float rb_fn_val(rb_string_t* s);
rb_string_t* rb_fn_ucase_s(rb_string_t* s);
rb_string_t* rb_fn_lcase_s(rb_string_t* s);
rb_string_t* rb_fn_trim_s(rb_string_t* s);

/* ── Math built-ins ──────────────────────────────────── */

float rb_fn_sqr(float x);
float rb_fn_abs(float x);
float rb_fn_sin(float x);
float rb_fn_cos(float x);
float rb_fn_tan(float x);
float rb_fn_atn(float x);
float rb_fn_log(float x);
float rb_fn_exp(float x);
int32_t rb_fn_int(float x);
int32_t rb_fn_fix(float x);
int32_t rb_fn_sgn(float x);
float rb_fn_rnd(void);

/* ── Assert ──────────────────────────────────────────── */

void rb_assert_fail(rb_string_t* message, int32_t offset) __attribute__((noreturn));

/* ── TRY/CATCH ───────────────────────────────────────── */

int32_t rb_try_begin(void);
void rb_try_end(void);
void rb_throw(rb_string_t* message);
rb_string_t* rb_get_error_message(void);

/* ── TASK (FreeRTOS / pthreads) ──────────────────────── */

void rb_task_create(void (*fn)(void*), rb_string_t* name, int32_t stack_size, int32_t priority);

/* ── EVENT system ────────────────────────────────────── */

void rb_on_gpio_change(int32_t pin, void (*handler)(void));
void rb_on_timer(int32_t interval_ms, void (*handler)(void));
void rb_on_mqtt_message(void (*handler)(void));

/* ── State Machine ───────────────────────────────────── */

int32_t rb_machine_create(const char* name);
void rb_machine_add_state(int32_t handle, const char* state_name);
void rb_machine_add_transition(int32_t handle, const char* from_state, const char* event_name, const char* to_state);
void rb_machine_event(int32_t handle, rb_string_t* event);
rb_string_t* rb_machine_get_state(int32_t handle);

/* ── NTP/Clock ───────────────────────────────────────── */

void rb_ntp_sync(rb_string_t* server);
rb_string_t* rb_ntp_time(void);
int32_t rb_ntp_epoch(void);

/* ── File System (LittleFS) ──────────────────────────── */

void rb_file_open(rb_string_t* path, rb_string_t* mode);
void rb_file_write(rb_string_t* data);
rb_string_t* rb_file_read(void);
void rb_file_close(void);
void rb_file_delete(rb_string_t* path);
int32_t rb_file_exists(rb_string_t* path);

/* ── WebSocket ───────────────────────────────────────── */

void rb_ws_connect(rb_string_t* url);
void rb_ws_send(rb_string_t* data);
rb_string_t* rb_ws_receive(void);
void rb_ws_close(void);

/* ── TCP Sockets ─────────────────────────────────────── */

void rb_tcp_listen(int32_t port);
int32_t rb_tcp_accept(void);
void rb_tcp_send(rb_string_t* data);
rb_string_t* rb_tcp_receive(void);
void rb_tcp_close(void);

/* ── Watchdog Timer ──────────────────────────────────── */

void rb_wdt_enable(int32_t timeout_ms);
void rb_wdt_feed(void);
void rb_wdt_disable(void);

/* ── HTTPS ───────────────────────────────────────────── */

rb_string_t* rb_https_get(rb_string_t* url);
rb_string_t* rb_https_post(rb_string_t* url, rb_string_t* body);

/* ── I2S Audio ───────────────────────────────────────── */

void rb_i2s_init(int32_t rate, int32_t bits, int32_t channels);
void rb_i2s_write(rb_string_t* data);
void rb_i2s_stop(void);

/* ── Web Server ──────────────────────────────────────── */
void rb_web_start(int32_t port);
rb_string_t* rb_web_wait(void);
rb_string_t* rb_web_body(void);
void rb_web_reply(int32_t status, rb_string_t* body);
void rb_web_stop(void);

/* ── SD Card ─────────────────────────────────────────── */
void rb_sd_init(int32_t cs_pin);
void rb_sd_open(rb_string_t* path, rb_string_t* mode);
void rb_sd_write(rb_string_t* data);
rb_string_t* rb_sd_read(void);
void rb_sd_close(void);
int32_t rb_sd_free(void);

/* ── Async / Yield ───────────────────────────────────── */
void rb_yield(void);
void rb_await(int32_t ms);

/* ── Cron ────────────────────────────────────────────── */
void rb_cron_add(int32_t id, rb_string_t* expr);
int32_t rb_cron_check(int32_t id);
void rb_cron_remove(int32_t id);

/* ── Regex ───────────────────────────────────────────── */
int32_t rb_regex_match(rb_string_t* pattern, rb_string_t* text);
rb_string_t* rb_regex_find(rb_string_t* pattern, rb_string_t* text);
rb_string_t* rb_regex_replace(rb_string_t* pattern, rb_string_t* text, rb_string_t* replacement);

/* ── Entry point (generated by compiler) ──────────────── */

extern void basic_program_entry(void);

#ifdef __cplusplus
}
#endif

#endif /* RB_RUNTIME_H */
