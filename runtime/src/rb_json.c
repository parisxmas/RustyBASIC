#include "rb_runtime.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

#ifdef ESP_PLATFORM
#include "cJSON.h"

/*
 * Navigate a cJSON tree by dot-separated path.
 * Supports keys and numeric indices, e.g. "data.items.0.name".
 */
static cJSON* json_get_by_path(cJSON* root, const char* path) {
    if (!root || !path) return NULL;

    char buf[256];
    strncpy(buf, path, sizeof(buf) - 1);
    buf[sizeof(buf) - 1] = '\0';

    cJSON* cur = root;
    char* token = strtok(buf, ".");
    while (token && cur) {
        if (cJSON_IsArray(cur)) {
            char* end;
            long idx = strtol(token, &end, 10);
            if (*end == '\0') {
                cur = cJSON_GetArrayItem(cur, (int)idx);
            } else {
                cur = cJSON_GetObjectItemCaseSensitive(cur, token);
            }
        } else {
            cur = cJSON_GetObjectItemCaseSensitive(cur, token);
        }
        token = strtok(NULL, ".");
    }
    return cur;
}
#endif

rb_string_t* rb_json_get(rb_string_t* json, rb_string_t* key) {
#ifdef ESP_PLATFORM
    if (!json || !key) return rb_string_alloc("");
    cJSON* root = cJSON_Parse(json->data);
    if (!root) return rb_string_alloc("");

    cJSON* item = json_get_by_path(root, key->data);
    rb_string_t* result;
    if (!item) {
        result = rb_string_alloc("");
    } else if (cJSON_IsString(item)) {
        result = rb_string_alloc(item->valuestring);
    } else {
        char* printed = cJSON_PrintUnformatted(item);
        result = rb_string_alloc(printed ? printed : "");
        if (printed) free(printed);
    }
    cJSON_Delete(root);
    return result;
#else
    printf("[JSON] get: json=%s, key=%s\n",
           json ? json->data : "(null)",
           key ? key->data : "(null)");
    return rb_string_alloc("");
#endif
}

rb_string_t* rb_json_set(rb_string_t* json, rb_string_t* key, rb_string_t* value) {
#ifdef ESP_PLATFORM
    if (!json || !key || !value) return rb_string_alloc("{}");

    cJSON* root = cJSON_Parse(json->data);
    if (!root) root = cJSON_CreateObject();

    /* Try parsing value as JSON first (for numbers, objects, arrays) */
    cJSON* val_json = cJSON_Parse(value->data);
    if (val_json) {
        cJSON_DeleteItemFromObjectCaseSensitive(root, key->data);
        cJSON_AddItemToObject(root, key->data, val_json);
    } else {
        cJSON_DeleteItemFromObjectCaseSensitive(root, key->data);
        cJSON_AddStringToObject(root, key->data, value->data);
    }

    char* printed = cJSON_PrintUnformatted(root);
    rb_string_t* result = rb_string_alloc(printed ? printed : "{}");
    if (printed) free(printed);
    cJSON_Delete(root);
    return result;
#else
    printf("[JSON] set: json=%s, key=%s, value=%s\n",
           json ? json->data : "(null)",
           key ? key->data : "(null)",
           value ? value->data : "(null)");
    return rb_string_alloc("{}");
#endif
}

int32_t rb_json_count(rb_string_t* json) {
#ifdef ESP_PLATFORM
    if (!json) return 0;
    cJSON* root = cJSON_Parse(json->data);
    if (!root) return 0;

    int32_t count = 0;
    if (cJSON_IsArray(root)) {
        count = cJSON_GetArraySize(root);
    } else if (cJSON_IsObject(root)) {
        cJSON* child = root->child;
        while (child) {
            count++;
            child = child->next;
        }
    }
    cJSON_Delete(root);
    return count;
#else
    printf("[JSON] count: json=%s\n", json ? json->data : "(null)");
    return 0;
#endif
}
