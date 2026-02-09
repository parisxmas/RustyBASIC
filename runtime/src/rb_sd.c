#include "rb_runtime.h"
#include <stdio.h>
#include <string.h>

#ifdef ESP_PLATFORM
#include "driver/sdspi_host.h"
#include "driver/spi_common.h"
#include "esp_vfs_fat.h"
#include "sdmmc_cmd.h"

static const char *mount_point = "/sdcard";
static FILE *current_file = NULL;

void rb_sd_init(int32_t cs_pin) {
    esp_vfs_fat_sdmmc_mount_config_t mount_config = {
        .format_if_mount_failed = false,
        .max_files = 5,
    };
    sdmmc_card_t *card;
    sdmmc_host_t host = SDSPI_HOST_DEFAULT();
    sdspi_device_config_t slot = SDSPI_DEVICE_CONFIG_DEFAULT();
    slot.gpio_cs = cs_pin;
    esp_vfs_fat_sdspi_mount(mount_point, &host, &slot, &mount_config, &card);
    printf("[SD] Initialized with CS pin %d\n", cs_pin);
}

void rb_sd_open(rb_string_t* path, rb_string_t* mode) {
    char full[320];
    snprintf(full, sizeof(full), "%s/%.*s", mount_point, path->len, path->data);
    char m[8];
    snprintf(m, sizeof(m), "%.*s", mode->len, mode->data);
    current_file = fopen(full, m);
}

void rb_sd_write(rb_string_t* data) {
    if (current_file) fwrite(data->data, 1, data->len, current_file);
}

rb_string_t* rb_sd_read(void) {
    if (!current_file) return rb_string_from_cstr("");
    char buf[4096];
    size_t n = fread(buf, 1, sizeof(buf) - 1, current_file);
    buf[n] = '\0';
    return rb_string_from_cstr(buf);
}

void rb_sd_close(void) {
    if (current_file) { fclose(current_file); current_file = NULL; }
}

int32_t rb_sd_free(void) {
    FATFS *fs;
    DWORD free_clust;
    f_getfree(mount_point, &free_clust, &fs);
    return (int32_t)(free_clust * fs->csize * 512);
}

#else

static FILE *current_file = NULL;

void rb_sd_init(int32_t cs_pin) {
    printf("[SD] Initialized with CS pin %d (stub)\n", cs_pin);
}

void rb_sd_open(rb_string_t* path, rb_string_t* mode) {
    char p[256], m[8];
    snprintf(p, sizeof(p), "./data/%.*s", path->len, path->data);
    snprintf(m, sizeof(m), "%.*s", mode->len, mode->data);
    current_file = fopen(p, m);
    printf("[SD] Open %s mode %s (stub)\n", p, m);
}

void rb_sd_write(rb_string_t* data) {
    if (current_file) fwrite(data->data, 1, data->len, current_file);
    printf("[SD] Write %d bytes (stub)\n", data->len);
}

rb_string_t* rb_sd_read(void) {
    if (!current_file) return rb_string_from_cstr("");
    char buf[4096];
    size_t n = fread(buf, 1, sizeof(buf) - 1, current_file);
    buf[n] = '\0';
    return rb_string_from_cstr(buf);
}

void rb_sd_close(void) {
    if (current_file) { fclose(current_file); current_file = NULL; }
    printf("[SD] File closed (stub)\n");
}

int32_t rb_sd_free(void) {
    printf("[SD] Free space query (stub)\n");
    return 1048576;
}

#endif
