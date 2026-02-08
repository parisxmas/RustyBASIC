#include "rb_runtime.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

#ifdef ESP_PLATFORM
#include "mqtt_client.h"
#include "freertos/FreeRTOS.h"
#include "freertos/queue.h"

#define MQTT_RECV_QUEUE_SIZE 8
#define MQTT_RECV_MSG_MAX   256
#define MQTT_RECV_TIMEOUT_MS 5000

static esp_mqtt_client_handle_t mqtt_client = NULL;
static QueueHandle_t mqtt_recv_queue = NULL;

typedef struct {
    char data[MQTT_RECV_MSG_MAX];
    int len;
} mqtt_msg_t;

static void mqtt_event_handler(void *handler_args, esp_event_base_t base,
                                int32_t event_id, void *event_data) {
    esp_mqtt_event_handle_t event = (esp_mqtt_event_handle_t)event_data;
    if (event_id == MQTT_EVENT_DATA && mqtt_recv_queue) {
        mqtt_msg_t msg;
        int copy_len = event->data_len;
        if (copy_len >= MQTT_RECV_MSG_MAX) {
            copy_len = MQTT_RECV_MSG_MAX - 1;
        }
        memcpy(msg.data, event->data, copy_len);
        msg.data[copy_len] = '\0';
        msg.len = copy_len;
        xQueueSend(mqtt_recv_queue, &msg, 0);
    }
}
#endif

void rb_mqtt_connect(rb_string_t* broker, int32_t port) {
#ifdef ESP_PLATFORM
    if (mqtt_client) {
        esp_mqtt_client_stop(mqtt_client);
        esp_mqtt_client_destroy(mqtt_client);
        mqtt_client = NULL;
    }
    if (!mqtt_recv_queue) {
        mqtt_recv_queue = xQueueCreate(MQTT_RECV_QUEUE_SIZE, sizeof(mqtt_msg_t));
    }
    esp_mqtt_client_config_t config = {
        .broker.uri = broker ? broker->data : "",
        .broker.port = (uint32_t)port,
    };
    mqtt_client = esp_mqtt_client_init(&config);
    esp_mqtt_client_register_event(mqtt_client, ESP_EVENT_ANY_ID,
                                    mqtt_event_handler, NULL);
    esp_mqtt_client_start(mqtt_client);
#else
    printf("[MQTT] connect: broker=%s, port=%d\n",
           broker ? broker->data : "(null)", (int)port);
#endif
}

void rb_mqtt_disconnect(void) {
#ifdef ESP_PLATFORM
    if (mqtt_client) {
        esp_mqtt_client_stop(mqtt_client);
        esp_mqtt_client_destroy(mqtt_client);
        mqtt_client = NULL;
    }
#else
    printf("[MQTT] disconnect\n");
#endif
}

void rb_mqtt_publish(rb_string_t* topic, rb_string_t* message) {
#ifdef ESP_PLATFORM
    if (mqtt_client) {
        esp_mqtt_client_publish(mqtt_client,
                                topic ? topic->data : "",
                                message ? message->data : "",
                                message ? message->length : 0,
                                0, 0);
    }
#else
    printf("[MQTT] publish: topic=%s, message=%s\n",
           topic ? topic->data : "(null)",
           message ? message->data : "(null)");
#endif
}

void rb_mqtt_subscribe(rb_string_t* topic) {
#ifdef ESP_PLATFORM
    if (mqtt_client) {
        esp_mqtt_client_subscribe(mqtt_client,
                                   topic ? topic->data : "",
                                   0);
    }
#else
    printf("[MQTT] subscribe: topic=%s\n",
           topic ? topic->data : "(null)");
#endif
}

rb_string_t* rb_mqtt_receive(void) {
#ifdef ESP_PLATFORM
    if (mqtt_recv_queue) {
        mqtt_msg_t msg;
        if (xQueueReceive(mqtt_recv_queue, &msg,
                          pdMS_TO_TICKS(MQTT_RECV_TIMEOUT_MS)) == pdTRUE) {
            return rb_string_alloc(msg.data);
        }
    }
    return rb_string_alloc("");
#else
    printf("[MQTT] receive (blocking)\n");
    return rb_string_alloc("");
#endif
}
