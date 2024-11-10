#mod gui "rustbee-gui"
#mod daemon "rustbee-daemon"

# Workaround because modules dont work as I would and the mods above were
# only for CLI calls at root. e.g. `just gui::build`
GUI := "just --justfile rustbee-gui/Justfile"
DAEMON := "just --justfile rustbee-daemon/Justfile"

export MSRV := "1.80"
export docker_base_pkgs := "(apt-get update && apt-get install -y libdbus-1-dev pkg-config) > /dev/null 2>&1"
log_path := `cat rustbee-common/src/constants.rs | grep "const LOG_PATH" | grep -oE '".*"' | sed s/\"//g`
socket_path := `cat rustbee-common/src/constants.rs | grep "const SOCKET_PATH" | grep -oE '".*"' | sed s/\"//g`
export purple := "\\e[35m"
export red := "\\e[31m"
export white := "\\e[0m"

alias ba := build-all
alias ia := install-all
alias help := _default

# Not actually called but when calling "just" it takes the first recipe
[doc]
_default:
    @just -l --justfile {{justfile()}} --list-heading $'Available commands:\n' --unsorted

@build-all: build build-gui

@install-all: install install-gui

# CLI Build
@build:
    cargo build --release
    sudo setcap cap_dac_override,cap_dac_read_search+ep ./target/release/rustbee
    echo -e "$purple[Rustbee CLI] Finished compiling !$white"
    {{DAEMON}} build

@build-gui:
    {{GUI}} build
    {{DAEMON}} build

[private]
@build-daemon:
    {{DAEMON}} build

# CLI Build
build-docker:
    #!/usr/bin/env bash
    if [[ -f ./target/release/rustbee && -f ./rustbee-daemon/target/release/rustbee-daemon ]]; then
        echo -e "${red}Binaries are already built, you can run \`just install-cli\` to install rustbee CLI$white"
        exit 1
    fi

    echo "Compiling Rustbee CLI & its daemon with docker... (feel free to make some coffe)"

    docker run --rm --user root -v {{justfile_dir()}}:/usr/src/rustbee -w /usr/src/rustbee rust:{{MSRV}}-bullseye \
        bash -c "{{docker_base_pkgs}} && cargo build --release && cd rustbee-daemon && cargo build --release"
    sudo chown -R $(id -u):$(id -g) ./target
    sudo chown -R $(id -u):$(id -g) ./rustbee-daemon/target

    sudo setcap cap_dac_override,cap_dac_read_search+ep ./target/release/rustbee
    sudo setcap cap_dac_override+ep ./rustbee-daemon/target/release/rustbee-daemon

    echo -e "${purple}Done! You can now run \`just install-cli\` to install rustbee CLI$white"

build-gui-docker:
    {{GUI}} build-docker

# Build C dynamic lib on rustbee-common/target/release/librustbee_common.so
@build-lib:
    cd rustbee-common && \
    cargo rustc --release --features ffi --crate-type=cdylib

@debug-gui:
    {{GUI}} debug

# Used for new releases, version is x.x.x without the 'v'
@push-ver version:
    sed '0,/version = ".*"/ s//version = "{{ version }}"/' -i rustbee-common/Cargo.toml
    git add rustbee-common/Cargo.toml
    find . -name Cargo.lock -execdir cargo update rustbee-common \;
    git add **/Cargo.lock
    git commit -m "docs(lib): Tag release v{{ version }}" # Cannot skip ci, it also skips the tag trigger
    git push origin main
    git tag -a v{{ version }} -m "Release v{{ version }}"
    git push origin --tags

install:
    #!/usr/bin/env bash
    if [[ ! -f ./target/release/rustbee || ! -f ./rustbee-daemon/target/release/rustbee-daemon ]]; then
        echo -e "${red}Binaries are not built, you must run \`just build-cli\` first$white"
        exit 1
    fi

    {{DAEMON}} install

    sudo ln -sf $PWD/target/release/rustbee /bin/rustbee
    echo -e "${purple}Rustbee CLI symlinked to /bin dir$white"
    echo -e "${purple}Done! You can now use \`rustbee\` globally$white"

@install-gui:
    {{DAEMON}} install
    {{GUI}} install

# This avoids failing so it can be used even if GUI isn't installed for e.g.
[doc("Optional flag: --preserve-logs")]
[confirm("Are you sure you want to uninstall everything ? (y/N)")]
@uninstall *flag:
    -cargo clean > /dev/null 2>&1
    -sudo rm /bin/rustbee > /dev/null 2>&1
    {{GUI}} uninstall
    {{DAEMON}} uninstall

    if {{ if flag == "--preserve-logs" { "true" } else { "false" } }}; then \
        -sudo rm $log_path > /dev/null 2>&1; \
    fi
    echo -e "${purple}Rustbee binaries are successfully uninstalled$white"
