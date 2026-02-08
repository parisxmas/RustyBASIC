#include "rb_runtime.h"
#include <stdio.h>

#ifdef ESP_PLATFORM
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

void rb_task_create(void (*fn)(void*), rb_string_t* name, int32_t stack_size, int32_t priority) {
    const char* task_name = (name && name->length > 0) ? name->data : "rb_task";
    xTaskCreate(fn, task_name, (uint32_t)stack_size, NULL, (UBaseType_t)priority, NULL);
}

#else
#include <pthread.h>

typedef struct {
    void (*fn)(void*);
} task_arg_t;

static void* task_wrapper(void* arg) {
    task_arg_t* ta = (task_arg_t*)arg;
    ta->fn(NULL);
    free(ta);
    return NULL;
}

void rb_task_create(void (*fn)(void*), rb_string_t* name, int32_t stack_size, int32_t priority) {
    (void)name;
    (void)stack_size;
    (void)priority;
    task_arg_t* ta = (task_arg_t*)malloc(sizeof(task_arg_t));
    ta->fn = fn;
    pthread_t thread;
    pthread_attr_t attr;
    pthread_attr_init(&attr);
    pthread_attr_setdetachstate(&attr, PTHREAD_CREATE_DETACHED);
    pthread_create(&thread, &attr, task_wrapper, ta);
    pthread_attr_destroy(&attr);
}

#endif
