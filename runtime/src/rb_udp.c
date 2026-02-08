#include "rb_runtime.h"
#include <stdio.h>

#ifdef ESP_PLATFORM
#include "lwip/sockets.h"
static int rb_udp_sock = -1;
#endif

void rb_udp_init(int32_t port) {
#ifdef ESP_PLATFORM
    rb_udp_sock = socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP);
    if (rb_udp_sock < 0) return;
    struct sockaddr_in addr = { .sin_family = AF_INET, .sin_port = htons((uint16_t)port), .sin_addr.s_addr = INADDR_ANY };
    bind(rb_udp_sock, (struct sockaddr*)&addr, sizeof(addr));
#else
    (void)port;
    fprintf(stderr, "[stub] UDP.INIT port=%d\n", (int)port);
#endif
}

void rb_udp_send(rb_string_t* host, int32_t port, rb_string_t* data) {
#ifdef ESP_PLATFORM
    if (rb_udp_sock < 0) rb_udp_init(0);
    struct sockaddr_in dest = { .sin_family = AF_INET, .sin_port = htons((uint16_t)port) };
    inet_aton(host->data, &dest.sin_addr);
    sendto(rb_udp_sock, data->data, data->length, 0, (struct sockaddr*)&dest, sizeof(dest));
#else
    fprintf(stderr, "[stub] UDP.SEND %s:%d \"%s\"\n", host ? host->data : "", (int)port, data ? data->data : "");
#endif
}

rb_string_t* rb_udp_receive(void) {
#ifdef ESP_PLATFORM
    char buf[1024];
    struct sockaddr_in src;
    socklen_t slen = sizeof(src);
    int n = recvfrom(rb_udp_sock, buf, sizeof(buf)-1, 0, (struct sockaddr*)&src, &slen);
    if (n <= 0) return rb_string_alloc("");
    buf[n] = '\0';
    return rb_string_alloc(buf);
#else
    fprintf(stderr, "[stub] UDP.RECEIVE not supported on host\n");
    return rb_string_alloc("");
#endif
}
