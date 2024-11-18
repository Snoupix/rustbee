# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)

## [Unreleased]

## [v0.1.0] - 2024-11-18

### Changed

- [lib] Breaking changes:
- - Bluetooth get_device(s) and search_devices_by_name will now return exclusively HueDevice<Server> variant since it's only supposed to be used server side
- - HueDevice addr field is now the raw [u8; 6] for interop

- [lib] Changes/Fixes:
- - Use of `bluest` for the Windows BTLE implementation and `btleplug` for Linux
- - Use of logger instead of stdout/stderr
- - BT streams will not return duplicates
- - Implementation of the launch/shutdown functions for Windows

### Added

- Script to prepare Windows install and set permissions on executables

### Fixed

- Fix FFI cross platform

## [v0.0.2] - 2024-11-10

### Changed

- log command => Can now log x last lines with the follow flag

### Added

- C dynamic library compiled and added to release

## [v0.0.1] - 2024-11-4

### Added

- CLI that features:
- - State storage (shared with GUI)
- - Logger (shared with GUI and Daemon) and log command to display them
- - (partial impl) Setting colors of the lights
- - Setting brightness percentage
- - Turning ON/OFF the lights
- - Disconnect from the lights
- - Launch/Shutdown the daemon
- GUI that features:
- - State storage (shared with CLI)
- - Logger (shared with CLI and Daemon)
- - Bluetooth discovery to retrieve/select lights
- - Display lights state
- - Setting brightness percentage
- - Turning ON/OFF the lights
- - Launch the daemon
- Daemon that features:
- - Self closing after x time without any communication
- - Logger (shared with CLI and Daemon)
- - Store the discovered lights for smoother experience on next commands
- - An async file socket/named pipe and handles/parses messages (non-blocking)
- [From source only] Rustbee-common library can be compiled to a C dynamic lib with a provided header file
