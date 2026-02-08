#include "rb_runtime.h"
#include <stdio.h>

#ifdef ESP_PLATFORM
#include "esp_ota_ops.h"
#include "esp_http_client.h"
#include "esp_https_ota.h"
#endif

void rb_ota_update(rb_string_t* url) {
#ifdef ESP_PLATFORM
    esp_http_client_config_t http_config = {
        .url = url ? url->data : "",
    };
    esp_https_ota_config_t ota_config = {
        .http_config = &http_config,
    };
    esp_err_t ret = esp_https_ota(&ota_config);
    if (ret == ESP_OK) {
        esp_restart();
    }
#else
    if (url) fprintf(stderr, "[stub] OTA.UPDATE %s\n", url->data);
#endif
}
