# Rustbee

Initially, this project was named to use the Zigbee protocol for my Philips Hue Play lights, but they implement the Bluetooth Low Energy (BLE) protocol, so I'm using it instead and keeping the name because it's a cool one.

## Goal

This CLI app aims to control my Hue lights without having to buy the *expensive* Philips Hue bridge and use a JSON based HTTP REST API and use BLE instead.

It will also later be used as a base for an [ESP32](https://www.espressif.com/en/products/socs/esp32) implementation to automate the control of my lights.

## Compatibility/Requirements

This project is built on [Arch linux](https://archlinux.org) *btw* thus it will not work on Windows neither WSL because the Bluetooth adapter is not (or hardly) available from WSL.

It might work on your Linux distro and maybe OSX if you have these required commands/programs (the others are "GNU/BSD defaults"):

- rustc >= 1.79
- rustup cargo component (rustup component add cargo)
- pgrep
- pkill
- bluetooth
- bluez
- bluezlibs
- bluezutils

<!-- TODO: Need to fix build_docker image first
Optional:
- rustc >= 1.79
- rustup cargo component (rustup component add cargo)
-->

## How to use

<!-- TODO: Need to fix build_docker image first
# If you don't have rust installed but you have docker you can use
./rustbee build_docker
-->

Note that you will need to enter your password, the bash script uses `sudo` to have permissions to create an IPC file socket at `/var/run` (which is root owned/protected)

```bash
# First, you need to build the binaries
./rustbee build

# Then, to get the CLI commands available
./rustbee help
```

*On error: if you get "le-connection-abort-by-local" error, it's kind of usual, BLE is a bit weak so try again your last command, it will most likely work after an other try*

### Modules/Binaries

1. Rustbee: The base module is used as the CLI bridge for light control (power state (set/get)/bightness (set/get)/color (set/get)/pair-trust/connect/disconnect)
1. Bluedaemon: The local filesystem socket running as a background daemon for interprocess communication (IPC) to keep connection with the lights and avoid connect/disconnect on every command (BLE communication is kind of tricky and fails sometimes) and disconnects them on a timeout

----

*this is probably the most over-engineered project I've made yet* :') *but I'm proud of it!*
