#include "rb_runtime.h"
#include <stdio.h>

#ifdef ESP_PLATFORM
#include "esp_websocket_client.h"

static esp_websocket_client_handle_t ws_client = NULL;
static char ws_rx_buf[1024];
static volatile int ws_rx_len = 0;

static void ws_event_handler(void *arg, esp_event_base_t base, int32_t id, void *data) {
    esp_websocket_event_data_t *ev = (esp_websocket_event_data_t *)data;
    if (id == WEBSOCKET_EVENT_DATA && ev->data_len < (int)sizeof(ws_rx_buf)) {
        memcpy(ws_rx_buf, ev->data_ptr, ev->data_len);
        ws_rx_buf[ev->data_len] = '\0';
        ws_rx_len = ev->data_len;
    }
}

void rb_ws_connect(rb_string_t* url) {
    esp_websocket_client_config_t cfg = { .uri = url->data };
    ws_client = esp_websocket_client_init(&cfg);
    esp_websocket_register_events(ws_client, WEBSOCKET_EVENT_ANY, ws_event_handler, NULL);
    esp_websocket_client_start(ws_client);
}

void rb_ws_send(rb_string_t* data) {
    if (ws_client) esp_websocket_client_send_text(ws_client, data->data, data->length, portMAX_DELAY);
}

rb_string_t* rb_ws_receive(void) {
    if (ws_rx_len > 0) { ws_rx_len = 0; return rb_string_alloc(ws_rx_buf); }
    return rb_string_alloc("");
}

void rb_ws_close(void) {
    if (ws_client) { esp_websocket_client_stop(ws_client); esp_websocket_client_destroy(ws_client); ws_client = NULL; }
}

#else

void rb_ws_connect(rb_string_t* url) { printf("[WS] connect %s\n", url->data); }
void rb_ws_send(rb_string_t* data) { printf("[WS] send: %s\n", data->data); }
rb_string_t* rb_ws_receive(void) { printf("[WS] receive\n"); return rb_string_alloc(""); }
void rb_ws_close(void) { printf("[WS] close\n"); }

#endif
