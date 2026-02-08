#include "rb_runtime.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

#ifdef ESP_PLATFORM
#include "esp_http_client.h"

static rb_string_t* http_perform(const char* url, const char* method,
                                  const char* post_data, int post_len) {
    esp_http_client_config_t config = {
        .url = url,
        .method = (strcmp(method, "POST") == 0) ? HTTP_METHOD_POST : HTTP_METHOD_GET,
    };
    esp_http_client_handle_t client = esp_http_client_init(&config);

    if (post_data && post_len > 0) {
        esp_http_client_set_post_field(client, post_data, post_len);
        esp_http_client_set_header(client, "Content-Type", "application/x-www-form-urlencoded");
    }

    esp_err_t err = esp_http_client_perform(client);
    rb_string_t* result;
    if (err == ESP_OK) {
        int len = esp_http_client_get_content_length(client);
        if (len <= 0) len = 0;
        char* buf = (char*)malloc(len + 1);
        if (buf) {
            esp_http_client_read(client, buf, len);
            buf[len] = '\0';
            result = rb_string_alloc(buf);
            free(buf);
        } else {
            result = rb_string_alloc("");
        }
    } else {
        result = rb_string_alloc("");
    }
    esp_http_client_cleanup(client);
    return result;
}
#endif

rb_string_t* rb_http_get(rb_string_t* url) {
#ifdef ESP_PLATFORM
    return http_perform(url ? url->data : "", "GET", NULL, 0);
#else
    printf("[HTTP] GET: url=%s\n", url ? url->data : "(null)");
    return rb_string_alloc("");
#endif
}

rb_string_t* rb_http_post(rb_string_t* url, rb_string_t* body) {
#ifdef ESP_PLATFORM
    return http_perform(url ? url->data : "", "POST",
                        body ? body->data : "", body ? body->length : 0);
#else
    printf("[HTTP] POST: url=%s, body=%s\n",
           url ? url->data : "(null)", body ? body->data : "(null)");
    return rb_string_alloc("");
#endif
}
