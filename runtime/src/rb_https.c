#include "rb_runtime.h"
#include <stdio.h>
#include <string.h>

#ifdef ESP_PLATFORM
#include "esp_http_client.h"
#include "esp_tls.h"

static char https_buf[4096];
static int https_buf_len = 0;

static esp_err_t https_event_handler(esp_http_client_event_t *evt) {
    if (evt->event_id == HTTP_EVENT_ON_DATA && !esp_http_client_is_chunked_response(evt->client)) {
        int space = sizeof(https_buf) - https_buf_len - 1;
        int copy = evt->data_len < space ? evt->data_len : space;
        memcpy(https_buf + https_buf_len, evt->data, copy);
        https_buf_len += copy;
    }
    return ESP_OK;
}

rb_string_t* rb_https_get(rb_string_t* url) {
    https_buf_len = 0;
    esp_http_client_config_t cfg = { .url = url->data, .event_handler = https_event_handler, .transport_type = HTTP_TRANSPORT_OVER_SSL };
    esp_http_client_handle_t client = esp_http_client_init(&cfg);
    esp_http_client_perform(client);
    esp_http_client_cleanup(client);
    https_buf[https_buf_len] = '\0';
    return rb_string_alloc(https_buf);
}

rb_string_t* rb_https_post(rb_string_t* url, rb_string_t* body) {
    https_buf_len = 0;
    esp_http_client_config_t cfg = { .url = url->data, .event_handler = https_event_handler, .transport_type = HTTP_TRANSPORT_OVER_SSL, .method = HTTP_METHOD_POST };
    esp_http_client_handle_t client = esp_http_client_init(&cfg);
    esp_http_client_set_post_field(client, body->data, body->length);
    esp_http_client_set_header(client, "Content-Type", "application/json");
    esp_http_client_perform(client);
    esp_http_client_cleanup(client);
    https_buf[https_buf_len] = '\0';
    return rb_string_alloc(https_buf);
}

#else

rb_string_t* rb_https_get(rb_string_t* url) {
    printf("[HTTPS] GET %s\n", url->data);
    return rb_string_alloc("{\"status\":\"ok\"}");
}

rb_string_t* rb_https_post(rb_string_t* url, rb_string_t* body) {
    printf("[HTTPS] POST %s body=%s\n", url->data, body->data);
    return rb_string_alloc("{\"status\":\"ok\"}");
}

#endif
