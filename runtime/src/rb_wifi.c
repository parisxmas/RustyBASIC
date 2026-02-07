#include "rb_runtime.h"
#include <stdio.h>
#include <string.h>

#ifdef ESP_PLATFORM
#include "esp_wifi.h"
#include "esp_event.h"
#include "nvs_flash.h"

static int wifi_initialized = 0;

static void ensure_wifi_init(void) {
    if (!wifi_initialized) {
        nvs_flash_init();
        esp_netif_init();
        esp_event_loop_create_default();
        esp_netif_create_default_wifi_sta();
        wifi_init_config_t cfg = WIFI_INIT_CONFIG_DEFAULT();
        esp_wifi_init(&cfg);
        esp_wifi_set_mode(WIFI_MODE_STA);
        wifi_initialized = 1;
    }
}
#endif

void rb_wifi_connect(rb_string_t* ssid, rb_string_t* password) {
#ifdef ESP_PLATFORM
    ensure_wifi_init();
    wifi_config_t wifi_config = {0};
    if (ssid && ssid->length < sizeof(wifi_config.sta.ssid)) {
        memcpy(wifi_config.sta.ssid, ssid->data, ssid->length);
    }
    if (password && password->length < sizeof(wifi_config.sta.password)) {
        memcpy(wifi_config.sta.password, password->data, password->length);
    }
    esp_wifi_set_config(WIFI_IF_STA, &wifi_config);
    esp_wifi_start();
    esp_wifi_connect();
#else
    printf("[WiFi] connect: ssid=%s\n", ssid ? ssid->data : "(null)");
#endif
}

int32_t rb_wifi_status(void) {
#ifdef ESP_PLATFORM
    wifi_ap_record_t ap_info;
    if (esp_wifi_sta_get_ap_info(&ap_info) == ESP_OK) {
        return 1;  /* connected */
    }
    return 0;  /* not connected */
#else
    printf("[WiFi] status check\n");
    return 0;
#endif
}

void rb_wifi_disconnect(void) {
#ifdef ESP_PLATFORM
    esp_wifi_disconnect();
    esp_wifi_stop();
#else
    printf("[WiFi] disconnect\n");
#endif
}
