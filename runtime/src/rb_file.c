#include "rb_runtime.h"
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>

#ifdef ESP_PLATFORM
#include "esp_littlefs.h"
#define FS_PREFIX "/littlefs"
#else
#define FS_PREFIX "./data"
#endif

static FILE* current_file = NULL;

void rb_file_open(rb_string_t* path, rb_string_t* mode) {
    if (current_file) fclose(current_file);
    char fullpath[256];
    snprintf(fullpath, sizeof(fullpath), "%s/%s", FS_PREFIX, path->data);
    current_file = fopen(fullpath, mode->data);
    if (!current_file) {
        printf("[FILE] failed to open %s\n", fullpath);
    }
}

void rb_file_write(rb_string_t* data) {
    if (current_file) {
        fwrite(data->data, 1, data->length, current_file);
    }
}

rb_string_t* rb_file_read(void) {
    if (!current_file) return rb_string_alloc("");
    char buf[1024];
    size_t n = fread(buf, 1, sizeof(buf) - 1, current_file);
    buf[n] = '\0';
    return rb_string_alloc(buf);
}

void rb_file_close(void) {
    if (current_file) {
        fclose(current_file);
        current_file = NULL;
    }
}

void rb_file_delete(rb_string_t* path) {
    char fullpath[256];
    snprintf(fullpath, sizeof(fullpath), "%s/%s", FS_PREFIX, path->data);
    remove(fullpath);
}

int32_t rb_file_exists(rb_string_t* path) {
    char fullpath[256];
    snprintf(fullpath, sizeof(fullpath), "%s/%s", FS_PREFIX, path->data);
    struct stat st;
    return (stat(fullpath, &st) == 0) ? -1 : 0;
}
