case $1 in
    "off") bluetooth off;;
    *) bluetooth on && cargo run $@;;
esac
