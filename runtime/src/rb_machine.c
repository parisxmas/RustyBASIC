#include "rb_runtime.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MAX_MACHINES 8
#define MAX_STATES 16
#define MAX_TRANSITIONS 64

typedef struct {
    char from_state[32];
    char event_name[32];
    char to_state[32];
} rb_transition_t;

typedef struct {
    char name[32];
    char states[MAX_STATES][32];
    int num_states;
    rb_transition_t transitions[MAX_TRANSITIONS];
    int num_transitions;
    int current_state;  /* index into states[] */
} rb_machine_t;

static rb_machine_t machines[MAX_MACHINES];
static int num_machines = 0;

int32_t rb_machine_create(const char* name) {
    if (num_machines >= MAX_MACHINES) {
        fprintf(stderr, "Too many state machines\n");
        return -1;
    }
    int handle = num_machines++;
    memset(&machines[handle], 0, sizeof(rb_machine_t));
    strncpy(machines[handle].name, name, 31);
    machines[handle].current_state = 0;
    return handle;
}

void rb_machine_add_state(int32_t handle, const char* state_name) {
    if (handle < 0 || handle >= num_machines) return;
    rb_machine_t* m = &machines[handle];
    if (m->num_states >= MAX_STATES) return;
    strncpy(m->states[m->num_states], state_name, 31);
    m->num_states++;
}

void rb_machine_add_transition(int32_t handle, const char* from_state, const char* event_name, const char* to_state) {
    if (handle < 0 || handle >= num_machines) return;
    rb_machine_t* m = &machines[handle];
    if (m->num_transitions >= MAX_TRANSITIONS) return;
    rb_transition_t* t = &m->transitions[m->num_transitions++];
    strncpy(t->from_state, from_state, 31);
    strncpy(t->event_name, event_name, 31);
    strncpy(t->to_state, to_state, 31);
}

static int find_state_index(rb_machine_t* m, const char* name) {
    for (int i = 0; i < m->num_states; i++) {
        if (strcmp(m->states[i], name) == 0) return i;
    }
    return -1;
}

void rb_machine_event(int32_t handle, rb_string_t* event) {
    if (handle < 0 || handle >= num_machines) return;
    if (!event || event->length == 0) return;
    rb_machine_t* m = &machines[handle];
    const char* current = m->states[m->current_state];

    for (int i = 0; i < m->num_transitions; i++) {
        rb_transition_t* t = &m->transitions[i];
        if (strcmp(t->from_state, current) == 0 && strcmp(t->event_name, event->data) == 0) {
            int new_state = find_state_index(m, t->to_state);
            if (new_state >= 0) {
                m->current_state = new_state;
                return;
            }
        }
    }
}

rb_string_t* rb_machine_get_state(int32_t handle) {
    if (handle < 0 || handle >= num_machines) {
        return rb_string_alloc("UNKNOWN");
    }
    return rb_string_alloc(machines[handle].states[machines[handle].current_state]);
}
