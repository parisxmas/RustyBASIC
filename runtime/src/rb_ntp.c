#include "rb_runtime.h"
#include <stdio.h>
#include <string.h>
#include <time.h>

#ifdef ESP_PLATFORM
#include "esp_sntp.h"

static bool ntp_synced = false;

static void ntp_sync_cb(struct timeval *tv) {
    ntp_synced = true;
}

void rb_ntp_sync(rb_string_t* server) {
    esp_sntp_setoperatingmode(SNTP_OPMODE_POLL);
    esp_sntp_setservername(0, server->data);
    sntp_set_time_sync_notification_cb(ntp_sync_cb);
    esp_sntp_init();
    int retry = 0;
    while (!ntp_synced && retry < 20) {
        vTaskDelay(pdMS_TO_TICKS(500));
        retry++;
    }
}

rb_string_t* rb_ntp_time(void) {
    time_t now;
    struct tm ti;
    char buf[64];
    time(&now);
    localtime_r(&now, &ti);
    strftime(buf, sizeof(buf), "%Y-%m-%d %H:%M:%S", &ti);
    return rb_string_alloc(buf);
}

int32_t rb_ntp_epoch(void) {
    return (int32_t)time(NULL);
}

#else /* Host stubs */

void rb_ntp_sync(rb_string_t* server) {
    printf("[NTP] sync with %s\n", server->data);
}

rb_string_t* rb_ntp_time(void) {
    time_t now = time(NULL);
    struct tm *ti = localtime(&now);
    char buf[64];
    strftime(buf, sizeof(buf), "%Y-%m-%d %H:%M:%S", ti);
    return rb_string_alloc(buf);
}

int32_t rb_ntp_epoch(void) {
    return (int32_t)time(NULL);
}

#endif
