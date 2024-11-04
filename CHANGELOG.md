# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)

## [Unreleased]

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
