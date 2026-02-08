#include "rb_runtime.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

#ifdef ESP_PLATFORM
#include "esp_now.h"
#include "esp_wifi.h"
#include "freertos/FreeRTOS.h"
#include "freertos/queue.h"

#define ESPNOW_RECV_QUEUE_SIZE 8
#define ESPNOW_RECV_MSG_MAX   250
#define ESPNOW_RECV_TIMEOUT_MS 5000

static QueueHandle_t espnow_recv_queue = NULL;
static bool espnow_inited = false;

typedef struct {
    char data[ESPNOW_RECV_MSG_MAX];
    int len;
} espnow_msg_t;

static void espnow_recv_cb(const esp_now_recv_info_t *info,
                            const uint8_t *data, int data_len) {
    if (espnow_recv_queue && data_len > 0) {
        espnow_msg_t msg;
        int copy_len = data_len;
        if (copy_len >= ESPNOW_RECV_MSG_MAX) {
            copy_len = ESPNOW_RECV_MSG_MAX - 1;
        }
        memcpy(msg.data, data, copy_len);
        msg.data[copy_len] = '\0';
        msg.len = copy_len;
        xQueueSend(espnow_recv_queue, &msg, 0);
    }
}

static int parse_mac(const char *str, uint8_t mac[6]) {
    unsigned int m[6];
    if (sscanf(str, "%x:%x:%x:%x:%x:%x",
               &m[0], &m[1], &m[2], &m[3], &m[4], &m[5]) != 6) {
        return -1;
    }
    for (int i = 0; i < 6; i++) mac[i] = (uint8_t)m[i];
    return 0;
}
#endif

void rb_espnow_init(void) {
#ifdef ESP_PLATFORM
    if (espnow_inited) return;
    if (!espnow_recv_queue) {
        espnow_recv_queue = xQueueCreate(ESPNOW_RECV_QUEUE_SIZE,
                                          sizeof(espnow_msg_t));
    }
    esp_now_init();
    esp_now_register_recv_cb(espnow_recv_cb);
    espnow_inited = true;
#else
    printf("[ESPNOW] init\n");
#endif
}

void rb_espnow_send(rb_string_t* peer, rb_string_t* data) {
#ifdef ESP_PLATFORM
    if (!espnow_inited) return;
    uint8_t mac[6];
    const char *peer_str = peer ? peer->data : "";
    if (parse_mac(peer_str, mac) != 0) {
        printf("[ESPNOW] invalid MAC: %s\n", peer_str);
        return;
    }
    /* Auto-add peer if not already added */
    esp_now_peer_info_t info = {0};
    memcpy(info.peer_addr, mac, 6);
    info.channel = 0;
    info.encrypt = false;
    if (!esp_now_is_peer_exist(mac)) {
        esp_now_add_peer(&info);
    }
    const char *msg = data ? data->data : "";
    esp_now_send(mac, (const uint8_t *)msg, strlen(msg));
#else
    printf("[ESPNOW] send: peer=%s, data=%s\n",
           peer ? peer->data : "(null)",
           data ? data->data : "(null)");
#endif
}

rb_string_t* rb_espnow_receive(void) {
#ifdef ESP_PLATFORM
    if (espnow_recv_queue) {
        espnow_msg_t msg;
        if (xQueueReceive(espnow_recv_queue, &msg,
                          pdMS_TO_TICKS(ESPNOW_RECV_TIMEOUT_MS)) == pdTRUE) {
            return rb_string_alloc(msg.data);
        }
    }
    return rb_string_alloc("");
#else
    printf("[ESPNOW] receive (blocking)\n");
    return rb_string_alloc("");
#endif
}
