#include "rb_runtime.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

#ifdef ESP_PLATFORM
#include "nimble/nimble_port.h"
#include "nimble/nimble_port_freertos.h"
#include "host/ble_hs.h"
#include "services/gap/ble_svc_gap.h"
#include "services/gatt/ble_svc_gatt.h"
#include "freertos/FreeRTOS.h"
#include "freertos/queue.h"

#define BLE_RECV_QUEUE_SIZE 8
#define BLE_RECV_MSG_MAX    256
#define BLE_RECV_TIMEOUT_MS 5000

static QueueHandle_t ble_recv_queue = NULL;
static uint16_t ble_conn_handle = 0;
static uint16_t ble_attr_handle = 0;
static bool ble_connected = false;

typedef struct {
    char data[BLE_RECV_MSG_MAX];
    int len;
} ble_msg_t;

/* GATT characteristic UUID: 0000ff01-0000-1000-8000-00805f9b34fb */
static const ble_uuid128_t gatt_chr_uuid =
    BLE_UUID128_INIT(0xfb, 0x34, 0x9b, 0x5f, 0x80, 0x00, 0x00, 0x80,
                     0x00, 0x10, 0x00, 0x00, 0x01, 0xff, 0x00, 0x00);

/* GATT service UUID: 0000ff00-0000-1000-8000-00805f9b34fb */
static const ble_uuid128_t gatt_svc_uuid =
    BLE_UUID128_INIT(0xfb, 0x34, 0x9b, 0x5f, 0x80, 0x00, 0x00, 0x80,
                     0x00, 0x10, 0x00, 0x00, 0x00, 0xff, 0x00, 0x00);

static int ble_chr_access(uint16_t conn_handle, uint16_t attr_handle,
                           struct ble_gatt_access_ctxt *ctxt, void *arg) {
    if (ctxt->op == BLE_GATT_ACCESS_OP_WRITE_CHR) {
        if (ble_recv_queue && ctxt->om) {
            ble_msg_t msg;
            uint16_t om_len = OS_MBUF_PKTLEN(ctxt->om);
            int copy_len = om_len < BLE_RECV_MSG_MAX - 1 ? om_len : BLE_RECV_MSG_MAX - 1;
            ble_hs_mbuf_to_flat(ctxt->om, msg.data, copy_len, NULL);
            msg.data[copy_len] = '\0';
            msg.len = copy_len;
            xQueueSend(ble_recv_queue, &msg, 0);
        }
        return 0;
    }
    return BLE_ATT_ERR_UNLIKELY;
}

static const struct ble_gatt_chr_def ble_chars[] = {
    {
        .uuid = &gatt_chr_uuid.u,
        .access_cb = ble_chr_access,
        .val_handle = &ble_attr_handle,
        .flags = BLE_GATT_CHR_F_READ | BLE_GATT_CHR_F_WRITE | BLE_GATT_CHR_F_NOTIFY,
    },
    { 0 },
};

static const struct ble_gatt_svc_def ble_svcs[] = {
    {
        .type = BLE_GATT_SVC_TYPE_PRIMARY,
        .uuid = &gatt_svc_uuid.u,
        .characteristics = ble_chars,
    },
    { 0 },
};

static void nimble_host_task(void *param) {
    (void)param;
    nimble_port_run();
}

static int ble_gap_event(struct ble_gap_event *event, void *arg) {
    switch (event->type) {
        case BLE_GAP_EVENT_CONNECT:
            if (event->connect.status == 0) {
                ble_conn_handle = event->connect.conn_handle;
                ble_connected = true;
            }
            break;
        case BLE_GAP_EVENT_DISCONNECT:
            ble_connected = false;
            break;
        default:
            break;
    }
    return 0;
}
#endif

void rb_ble_init(rb_string_t* name) {
#ifdef ESP_PLATFORM
    if (!ble_recv_queue) {
        ble_recv_queue = xQueueCreate(BLE_RECV_QUEUE_SIZE, sizeof(ble_msg_t));
    }
    nimble_port_init();
    ble_svc_gap_device_name_set(name ? name->data : "RustyBASIC");
    ble_svc_gap_init();
    ble_svc_gatt_init();
    ble_gatts_count_cfg(ble_svcs);
    ble_gatts_add_svcs(ble_svcs);
    nimble_port_freertos_init(nimble_host_task);
#else
    printf("[BLE] init: name=%s\n", name ? name->data : "(null)");
#endif
}

void rb_ble_advertise(int32_t mode) {
#ifdef ESP_PLATFORM
    struct ble_gap_adv_params adv_params = {0};
    if (mode) {
        adv_params.conn_mode = BLE_GAP_CONN_MODE_UND;
        adv_params.disc_mode = BLE_GAP_DISC_MODE_GEN;
        ble_gap_adv_start(BLE_OWN_ADDR_PUBLIC, NULL, BLE_HS_FOREVER,
                          &adv_params, ble_gap_event, NULL);
    } else {
        ble_gap_adv_stop();
    }
#else
    printf("[BLE] advertise: mode=%d\n", (int)mode);
#endif
}

rb_string_t* rb_ble_scan(void) {
#ifdef ESP_PLATFORM
    /* Scan returns first discovered device name as a simple string */
    printf("[BLE] scan (not fully implemented — use advertise mode)\n");
    return rb_string_alloc("");
#else
    printf("[BLE] scan\n");
    return rb_string_alloc("");
#endif
}

void rb_ble_send(rb_string_t* data) {
#ifdef ESP_PLATFORM
    if (ble_connected && ble_attr_handle) {
        struct os_mbuf *om = ble_hs_mbuf_from_flat(
            data ? data->data : "", data ? data->length : 0);
        ble_gatts_notify_custom(ble_conn_handle, ble_attr_handle, om);
    }
#else
    printf("[BLE] send: data=%s\n", data ? data->data : "(null)");
#endif
}

rb_string_t* rb_ble_receive(void) {
#ifdef ESP_PLATFORM
    if (ble_recv_queue) {
        ble_msg_t msg;
        if (xQueueReceive(ble_recv_queue, &msg,
                          pdMS_TO_TICKS(BLE_RECV_TIMEOUT_MS)) == pdTRUE) {
            return rb_string_alloc(msg.data);
        }
    }
    return rb_string_alloc("");
#else
    printf("[BLE] receive (blocking)\n");
    return rb_string_alloc("");
#endif
}
