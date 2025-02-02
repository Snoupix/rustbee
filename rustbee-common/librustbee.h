#ifndef DEVICE_H
#define DEVICE_H
#ifdef __cplusplus
extern "C" {
#endif

#include <stdlib.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define ADDR_LEN 6

typedef struct _device {
    uint8_t addr[ADDR_LEN];
    uint8_t _unused[58];
} Device;

Device* new_device(const uint8_t (*)[ADDR_LEN]);

bool try_connect(Device*);
bool try_disconnect(Device*);

bool set_power(Device*, uint8_t);
bool set_brightness(Device*, uint8_t);
bool set_color_rgb(Device*, uint8_t, uint8_t, uint8_t);

bool get_power(Device*);
uint8_t get_brightness(Device*);
uint8_t (*get_name(Device*))[19];
uint8_t (*get_color_rgb(Device*))[3];

bool launch_daemon();
// Optional since the daemon closes itself after a timeout
// without requests
bool shutdown_daemon(uint8_t);

void free_device(Device*);
void free_name(uint8_t (*)[19]);
void free_color_rgb(uint8_t (*)[3]);

#ifdef __cplusplus
}
#endif
#endif
