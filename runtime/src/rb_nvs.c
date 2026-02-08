#include "rb_runtime.h"
#include <stdio.h>

#ifdef ESP_PLATFORM
#include "nvs_flash.h"
#include "nvs.h"

static int nvs_initialized = 0;

static void ensure_nvs_init(void) {
    if (!nvs_initialized) {
        nvs_flash_init();
        nvs_initialized = 1;
    }
}
#endif

void rb_nvs_write(rb_string_t* key, int32_t value) {
#ifdef ESP_PLATFORM
    ensure_nvs_init();
    nvs_handle_t handle;
    if (nvs_open("rb_storage", NVS_READWRITE, &handle) == ESP_OK) {
        nvs_set_i32(handle, key ? key->data : "", value);
        nvs_commit(handle);
        nvs_close(handle);
    }
#else
    printf("[NVS] write: key=%s, value=%d\n",
           key ? key->data : "(null)", (int)value);
#endif
}

int32_t rb_nvs_read(rb_string_t* key) {
#ifdef ESP_PLATFORM
    ensure_nvs_init();
    nvs_handle_t handle;
    int32_t value = 0;
    if (nvs_open("rb_storage", NVS_READONLY, &handle) == ESP_OK) {
        nvs_get_i32(handle, key ? key->data : "", &value);
        nvs_close(handle);
    }
    return value;
#else
    printf("[NVS] read: key=%s\n", key ? key->data : "(null)");
    return 0;
#endif
}
