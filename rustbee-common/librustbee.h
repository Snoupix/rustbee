#ifndef DEVICE_H
#define DEVICE_H

#include <stdlib.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

typedef struct _device {
    uint8_t addr[6];
    uint8_t _unused[58];
} Device;

Device* new_device(const uint8_t[6]);
void free_device(Device*);

bool try_connect(Device*);
bool try_disconnect(Device*);

bool set_power(Device*, const uint8_t*);
bool set_brightness(Device*, const uint8_t*);

const uint8_t* get_brightness(Device*);

bool launch_daemon();
// Optional since the daemon closes itself after a timeout
// without requests
bool shutdown_daemon(const uint8_t*);

#endif
