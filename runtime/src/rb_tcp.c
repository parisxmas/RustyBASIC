#include "rb_runtime.h"
#include <stdio.h>
#include <string.h>

#ifdef ESP_PLATFORM
#include "lwip/sockets.h"

static int tcp_server_fd = -1;
static int tcp_client_fd = -1;

void rb_tcp_listen(int32_t port) {
    tcp_server_fd = socket(AF_INET, SOCK_STREAM, 0);
    struct sockaddr_in addr = { .sin_family = AF_INET, .sin_port = htons(port), .sin_addr.s_addr = INADDR_ANY };
    int opt = 1;
    setsockopt(tcp_server_fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));
    bind(tcp_server_fd, (struct sockaddr*)&addr, sizeof(addr));
    listen(tcp_server_fd, 1);
}

int32_t rb_tcp_accept(void) {
    if (tcp_server_fd < 0) return -1;
    tcp_client_fd = accept(tcp_server_fd, NULL, NULL);
    return tcp_client_fd;
}

void rb_tcp_send(rb_string_t* data) {
    if (tcp_client_fd >= 0) send(tcp_client_fd, data->data, data->length, 0);
}

rb_string_t* rb_tcp_receive(void) {
    char buf[1024];
    if (tcp_client_fd < 0) return rb_string_alloc("");
    int n = recv(tcp_client_fd, buf, sizeof(buf) - 1, 0);
    if (n <= 0) return rb_string_alloc("");
    buf[n] = '\0';
    return rb_string_alloc(buf);
}

void rb_tcp_close(void) {
    if (tcp_client_fd >= 0) { close(tcp_client_fd); tcp_client_fd = -1; }
    if (tcp_server_fd >= 0) { close(tcp_server_fd); tcp_server_fd = -1; }
}

#else
#include <sys/socket.h>
#include <netinet/in.h>
#include <unistd.h>

static int tcp_server_fd = -1;
static int tcp_client_fd = -1;

void rb_tcp_listen(int32_t port) {
    tcp_server_fd = socket(AF_INET, SOCK_STREAM, 0);
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons(port);
    addr.sin_addr.s_addr = INADDR_ANY;
    int opt = 1;
    setsockopt(tcp_server_fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));
    bind(tcp_server_fd, (struct sockaddr*)&addr, sizeof(addr));
    listen(tcp_server_fd, 1);
    printf("[TCP] listening on port %d\n", port);
}

int32_t rb_tcp_accept(void) {
    if (tcp_server_fd < 0) return -1;
    tcp_client_fd = accept(tcp_server_fd, NULL, NULL);
    printf("[TCP] accepted client fd=%d\n", tcp_client_fd);
    return tcp_client_fd;
}

void rb_tcp_send(rb_string_t* data) {
    if (tcp_client_fd >= 0) send(tcp_client_fd, data->data, data->length, 0);
}

rb_string_t* rb_tcp_receive(void) {
    char buf[1024];
    if (tcp_client_fd < 0) return rb_string_alloc("");
    ssize_t n = recv(tcp_client_fd, buf, sizeof(buf) - 1, 0);
    if (n <= 0) return rb_string_alloc("");
    buf[n] = '\0';
    return rb_string_alloc(buf);
}

void rb_tcp_close(void) {
    if (tcp_client_fd >= 0) { close(tcp_client_fd); tcp_client_fd = -1; }
    if (tcp_server_fd >= 0) { close(tcp_server_fd); tcp_server_fd = -1; }
}

#endif
