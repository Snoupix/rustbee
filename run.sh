#!/bin/bash

case $1 in
    "off") bluetooth off;;
    *)
        bluetooth | rg "bluetooth = off" > /dev/null 2>&1
        if [[ $? == 0 ]]; then
            bluetooth on > /dev/null 2>&1 && sleep 2
        fi

        cargo run $@;;
esac
