# Rustbee

Initially, this project was named to use the Zigbee protocol for my Philips Hue Play lights, but they implement the Bluetooth Low Energy (BLE) protocol, so I'm using it instead and keeping the name because it's a cool one.

## Goal

This project aims to control my Philips Hue lights without having to buy the *expensive* Philips Hue bridge and use a JSON based HTTP REST API instead of BLE.

It will also later be used as a base for an [ESP32](https://www.espressif.com/en/products/socs/esp32) implementation to automate the control of my lights.

## Disclaimer

*While this section exists, it means that this project is in Work In Progress. I will do my best to push working states on the main branch but there will probably still have some kind of issues.*

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
- rustup cargo component (`rustup component add cargo`)

## Build from source

This project uses [just](https://github.com/casey/just) as a command runner, which is a `make` alternative. It's recommanded to have it installed to build the project from source.

Note that you will need to enter your password, the daemon needs `sudo` to have permissions to create an IPC file socket at `/var/run` (which is root owned/protected), the log file at `/var/log` and if you're building with docker, it will compile root owned binaries so it will need your password at the end to change owner of these files.

Also, the daemon is compiled whether you chose the CLI or GUI.

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

# If you want to uninstall
just uninstall
# And you can also add "--preserve-logs" to avoid removing logs file
```

## How to use

To connect a Philips Hue light, you may need the official mobile app [Apple Store](https://apps.apple.com/us/app/philips-hue-gen-2/id1055281310?ls=1) - [Google Play Store](https://play.google.com/store/apps/details?id=com.philips.lighting.hue2). On the app, you need to go to `Settings > Voice Assistants > Amazon Alexa and tap Make visible` [thanks to alexhorn/libhueble](https://github.com/alexhorn/libhueble/issues/1).

This will enable the device to be discoverable and then after that, you will have to **pair** and **trust** your device via Bluetooth.

If you can't try again but factory reset your light before.

```bash
# To get the CLI commands available
rustbee help
# Or you can launch the GUI
rustbee gui

# You can get the logs file path and use it as you wish (e.g. `just logs | xargs cat` or `tail $(just logs)`)
just logs
# But it will soon be an added command to the CLI with options like --limit and --follow
# rustbee logs
```

*On error: if you get "le-connection-abort-by-local" error, it's kind of usual, BLE is a bit weak so try again your last command, it will most likely work after an other try*

### Modules/Binaries

1. **rustbee** (bin): The base module is used as the CLI (Command Line Interface) for light control features: power state (set/get), bightness (set/get), color (set/get), shutdown (and disconnect), gui (to launch the GUI)
1. **rustbee-gui** (bin): The GUI (Graphical User Interface) that can replace the CLI for a better UX and will also be a WASM module to use the browser instead of native GUI
1. **rustbee-daemon** (bin): The local filesystem socket running as a background daemon for interprocess communication (IPC) to keep connection with the lights and avoid connect/disconnect on every command (BLE communication is kind of tricky and fails sometimes) and disconnects them on a timeout
1. **rustbee-common** (lib): Actual implementations of bluetooth devices and common stuff used by the other binaries

### TODO

- [ ] Migrate from unix domain socket to local_socket for interop
- [x] Migrate from bluez to bleplug for interop (lost pair and trust features on the process)
- [ ] Clarify CLI args (add descriptions)
- [ ] When finished with GUI impl, try to impl WASM build target
- [ ] Impl CLI data lights save and maybe share it with GUI
- [ ] [CLI] Find a way to select a device with a better UX
- [x] Impl `justfile` recipes to replace bash script for a better DX and update README for steps
- - [x] `rustbee gui` should launch the gui executable
- - [x] The deamon launch feature should be migrated to common so cli and gui can launch it without bash
- - [x] setcap of rustbee cli exec to be able to create file socket
- - [x] setcap of rustbee daemon exec to be able to create log file
- [ ] Impl a better logging for the daemon and it should log to file itself
- [ ] CLI should have a logs command to output the log file to stdout
- [ ] Impl CI to create and publish binaries

----

*this is probably the most over-engineered project I've made yet* :') *but I'm proud of it!*
