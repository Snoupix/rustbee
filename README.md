# Rustbee

Initially, this project was named to use the Zigbee protocol for my Philips Hue Play lights, but they implement the Bluetooth Low Energy (BLE) protocol, so I'm using it instead and keeping the name because it's a cool one.

## Goal

This project aims to control my Philips Hue lights without having to buy the *expensive* Philips Hue bridge and use a JSON based HTTP REST API instead of BLE.

It will also later be used as a base for an [ESP32](https://www.espressif.com/en/products/socs/esp32) implementation to automate the control of my lights.

## Compatibility/Requirements

This project is built on [Arch linux](https://archlinux.org) *btw* thus it will not work on Windows neither WSL because the Bluetooth adapter is not (or hardly) available from WSL.

It might work on your Linux distro and maybe OSX if you have these required commands/programs (the others are "GNU/BSD defaults"):

- pgrep
- pkill
- bluetooth
- bluez
- bluezlibs
- bluezutils

Optional if docker is installed and you wanna compile locally:
- rustc >= 1.80
- rustup cargo component (rustup component add cargo)

## How to use

Note that you will need to enter your password, the bash script uses `sudo` to have permissions to create an IPC file socket at `/var/run` (which is root owned/protected), the log file at `/var/log` and if you're building with docker, it will compile root owned binaries so it will need your password at the end to change owner of these files.

Depending on your CPU, compiling this project may take you around 2mins with or without docker.

```bash
# First, you need to build the binaries
./rustbee build cli
# Or, if you prefer using the GUI
./rustbee build gui
# And you can also use the -d or --docker flag at the end of the command to use docker to compile

# Then, to get the CLI commands available
./rustbee help
# Or you can launch the GUI
./rustbee gui

# If you just wanna stop the rustbee-daemon manually and close (delete) the file socket
./rustbee shutdown

# (rust/cargo required) If you wanna save space and not use the app anymore
./rustbee clean_binaries
```

*On error: if you get "le-connection-abort-by-local" error, it's kind of usual, BLE is a bit weak so try again your last command, it will most likely work after an other try*

### Modules/Binaries

1. rustbee (bin): The base module is used as the CLI (Command Line Interface) for light control (power state (set/get)/bightness (set/get)/color (set/get)/pair-trust/connect/disconnect)
1. rustbee-daemon (bin): The local filesystem socket running as a background daemon for interprocess communication (IPC) to keep connection with the lights and avoid connect/disconnect on every command (BLE communication is kind of tricky and fails sometimes) and disconnects them on a timeout
1. rustbee-common (lib): Actual implementations of bluetooth devices and common stuff used by the other binaries
1. rustbee-gui (bin): The GUI (Graphical User Interface) that can replace the CLI for a better UX and will also be a WASM module to use the browser instead of natiive GUI

### TODO
- [ ] Migrate from unix domain socket to local_socket for interop
- [ ] Migrate from bluez to bleplug for interop

----

*this is probably the most over-engineered project I've made yet* :') *but I'm proud of it!*
