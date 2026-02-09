#include "rb_runtime.h"
#include <stdio.h>

#ifdef ESP_PLATFORM
#include "esp_http_server.h"

static httpd_handle_t server = NULL;
static char last_uri[256] = "";
static char last_body[4096] = "";
static httpd_req_t* pending_req = NULL;

static esp_err_t handler(httpd_req_t *req) {
    strncpy(last_uri, req->uri, sizeof(last_uri) - 1);
    int len = req->content_len;
    if (len > 0 && len < (int)sizeof(last_body)) {
        httpd_req_recv(req, last_body, len);
        last_body[len] = '\0';
    } else {
        last_body[0] = '\0';
    }
    pending_req = req;
    return ESP_OK;
}

void rb_web_start(int32_t port) {
    httpd_config_t config = HTTPD_DEFAULT_CONFIG();
    config.server_port = port;
    httpd_start(&server, &config);
    httpd_uri_t uri = { .uri = "/*", .method = HTTP_GET, .handler = handler };
    httpd_register_uri_handler(server, &uri);
    printf("[WEB] Server started on port %d\n", port);
}

rb_string_t* rb_web_wait(void) {
    while (pending_req == NULL) { vTaskDelay(pdMS_TO_TICKS(10)); }
    return rb_string_from_cstr(last_uri);
}

rb_string_t* rb_web_body(void) {
    return rb_string_from_cstr(last_body);
}

void rb_web_reply(int32_t status, rb_string_t* body) {
    if (pending_req) {
        char stat[16];
        snprintf(stat, sizeof(stat), "%d", status);
        httpd_resp_set_status(pending_req, stat);
        httpd_resp_send(pending_req, body->data, body->len);
        pending_req = NULL;
    }
}

void rb_web_stop(void) {
    if (server) { httpd_stop(server); server = NULL; }
    printf("[WEB] Server stopped\n");
}

#else

void rb_web_start(int32_t port) {
    printf("[WEB] Server started on port %d (stub)\n", port);
}

rb_string_t* rb_web_wait(void) {
    printf("[WEB] Waiting for request (stub)\n");
    return rb_string_from_cstr("/index.html");
}

rb_string_t* rb_web_body(void) {
    printf("[WEB] Get body (stub)\n");
    return rb_string_from_cstr("");
}

void rb_web_reply(int32_t status, rb_string_t* body) {
    printf("[WEB] Reply %d: %.*s (stub)\n", status, body->len, body->data);
}

void rb_web_stop(void) {
    printf("[WEB] Server stopped (stub)\n");
}

#endif
