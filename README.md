# Rustbee

Initially, this project was named to use the Zigbee protocol for my Philips Hue Play lights, but they implement the Bluetooth Low Energy (BLE) protocol, so I'm using it instead and keeping the name because it's a cool one.

## Goal

This project aims to control Philips Hue lights without having to buy the *expensive* Philips Hue bridge and use a JSON based HTTP REST API instead of BLE.

*Yes, BLE is not as optimized as a Zigbee network but for the moment, this project is BLE based.*

It will also later be used as a base for an [ESP32](https://www.espressif.com/en/products/socs/esp32) implementation to automate the control of my lights.

## State

This project is not stable *yet* thus, some features may not work (at all).

## How to install

You have two options to install and use Rustbee:
- Build the project yourself from source by following [those steps](#build-from-source).
- Use the compiled release and follow those steps.

You can check on the [latest release](https://github.com/Snoupix/rustbee/releases/latest) page and get either the CLI or GUI or both.
If there are no pre-built binaries for your OS/arch, it means that it is not *currently* supported and **may** never be. *You can still try to build it from source yourself though.*

The compressed folder contains instructions on how to properly install Rustbee and the 2 executables: The CLI/GUI and the daemon.

## How to use

To connect a Philips Hue light, you may need the official mobile app [Apple Store](https://apps.apple.com/us/app/philips-hue-gen-2/id1055281310?ls=1) - [Google Play Store](https://play.google.com/store/apps/details?id=com.philips.lighting.hue2). On the app, you need to go to `Settings > Voice Assistants > Amazon Alexa and tap Make visible` [thanks to alexhorn/libhueble](https://github.com/alexhorn/libhueble/issues/1).

This will enable the device to be discoverable and then after that, you will have to **pair** and **trust** your device via Bluetooth.

If you can't, try again but factory reset your light before.

```bash
# To get the CLI commands available
rustbee help
rustbee [command] help
# Or you can launch the GUI
rustbee gui

# e.g. this command will use these 2 MAC addresses to find the devices,
# turn them ON, save them to local storage file (so you don't have to specify
# them on next commands) and shutdown the daemon after the command
rustbee power on -s1a e8:d4:ea:c4:62:00 ec:27:a7:d6:5a:9c
```

*Known error: if you have an error with: "le-connection-abort-by-local", it's kind of usual, BLE is a bit weak so try again your last command, it will most likely work after an other try*

If you have any other issue, don't hesitate to [create an issue](https://github.com/Snoupix/rustbee/issues/new). An issue template doesn't exists yet so please, be as clear as you can.

## Build from source

<details>
<summary>Expand to see more</summary>

### Compatibility/Requirements

This project is built on [Arch linux](https://archlinux.org) *btw* and I'm making my best to make it Windows compatible but it might not or partially work on anything other than Linux.

I'm using [just](https://github.com/casey/just) as a command runner, which is a `make` alternative. It's highly recommanded to have it installed to build the project from source.

Note that you will need to enter your password, the just recipe needs `sudo` to have permissions to give capabilities to the daemon so it can create an IPC file socket at `/var/run` (which is root owned/protected), the log file at `/var/log` and if you're building with docker, it will compile root owned binaries so it will need your password at the end to change owner of these files too.

*Also, the daemon is compiled whether you chose the CLI or GUI.*

Rust requirements:
- rustc >= 1.80
- rustup cargo component (`rustup component add cargo`)

Or, you can use [Docker](https://www.docker.com/) to build the project if you don't have Rust installed locally.

### Build steps

Use `just` or `just help` to retrieve available commands or follow those steps:

```bash
# First, you need to build the binaries
just build
# Or, if you prefer using the GUI
just build-gui
# And if you don't have rustc/cargo you can also use docker to compile the CLI
just build-docker
# Or to compile the GUI
just build-gui-docker

# Then, you can manually use the executables or let rustbee add a symlink
# to the binaries on /bin for you
just install
# Or, for the GUI
just install-gui

# If you just want to stop the rustbee-daemon manually and close (delete)
# the file socket and if for some reason it doesn't kill the process
# gracefully, you can use -f or --force to force kill the daemon and
# if it outputs "Permission denied (os error 13)" you have to use sudo
# with the --force flag
just shutdown

# If you want to uninstall (remove the symlink to /bin and erase binaries and their build deps)
just uninstall
# And you can also add "--preserve-logs" to avoid removing logs file
```
</details>

### Modules/Binaries

1. **rustbee** (bin): The base module is used as the CLI (Command Line Interface) for light control features: power state (set/get), bightness (set/get), color (set/get), shutdown (and disconnect), gui (to launch the GUI)
1. **rustbee-gui** (bin): The GUI (Graphical User Interface) that can replace the CLI for a better UX and will also be a WASM module to use the browser instead of native GUI
1. **rustbee-daemon** (bin): The local filesystem socket running as a background daemon for interprocess communication (IPC) to keep connection with the lights and avoid connect/disconnect on every command (BLE communication is kind of tricky and fails sometimes) and disconnects them on a timeout
1. **rustbee-common** (lib): Actual implementations of bluetooth devices and common stuff used by the other binaries. It can also be compiled to a C dynamic lib (C header included) to use Rustbee features with any other C compatible languages !

----

*this is probably the most over-engineered project I've made yet* :') *but I'm proud of it!*
