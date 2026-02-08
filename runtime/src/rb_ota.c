#include "rb_runtime.h"
#include <stdio.h>

#ifdef ESP_PLATFORM
#include "esp_ota_ops.h"
#include "esp_http_client.h"
#include "esp_https_ota.h"
#endif

void rb_ota_update(rb_string_t* url) {
#ifdef ESP_PLATFORM
    esp_http_client_config_t config = {
        .url = url->data,
    };
    esp_err_t ret = esp_https_ota(&config);
    if (ret == ESP_OK) {
        esp_restart();
    }
#else
    if (url) fprintf(stderr, "[stub] OTA.UPDATE %s\n", url->data);
#endif
}
