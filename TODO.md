### TODO

- [x] Impl interop for the daemon socket (path...)
- [x] Migrate from bluez to bleplug for interop (lost pair and trust features on the process)
- [x] Clarify CLI args (add descriptions)
- [ ] When finished with GUI impl, try to impl WASM build target
- [x] Impl CLI data lights save and maybe share it with GUI
- - [x] Impl shared Storage using ~eframe::Storage trait~ (didn't because it links String to String and I would parse data all around)
- [ ] [CLI] Find a way to select a device with a better UX ?
- [x] Impl `justfile` recipes to replace bash script for a better DX and update README for steps
- - [x] `rustbee gui` should launch the gui executable
- - [x] The deamon launch feature should be migrated to common so cli and gui can launch it without bash
- - [x] setcap of rustbee cli exec to be able to create file socket
- - [x] setcap of rustbee daemon exec to be able to create log file
- [x] Impl a better logging for the daemon and it should log to file itself
- [x] CLI should have a logs command to output the log file to stdout
- [x] Impl CI to create and publish binaries on v* tag creation
- [ ] Impl CHANGES.md and INSTRUCTIONS.txt files for the release on v1.0.0 and push a tag when v1 is out so the release action is triggered automatically (also, change the changes-file field on the CI).
- [ ] Impl unit and integration tests
- [ ] Add a C dyn lib to pre-built releases with the CI and add the header file