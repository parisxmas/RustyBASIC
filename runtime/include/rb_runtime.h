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

/* ── Entry point (generated by compiler) ──────────────── */

extern void basic_program_entry(void);

#ifdef __cplusplus
}
#endif

#endif /* RB_RUNTIME_H */
