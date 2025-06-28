// This particular file has been partially AI generated so keep that in mind for your own implementation

const symbols = {
    new_device: {
        parameters: ["pointer"], // Pointer to an array of ADDR_LEN bytes
        result: "pointer", // Returns a pointer to a Device struct
    },
    try_connect: {
        parameters: ["pointer"], // Device pointer
        result: "bool", // Returns a boolean
    },
    try_disconnect: {
        parameters: ["pointer"],
        result: "bool",
    },
    set_power: {
        parameters: ["pointer", "u8"],
        result: "bool",
    },
    set_brightness: {
        parameters: ["pointer", "u8"],
        result: "bool",
    },
    get_power: {
        parameters: ["pointer"],
        result: "bool",
    },
    get_brightness: {
        parameters: ["pointer"],
        result: "u8",
    },
    get_name: {
        parameters: ["pointer"],
        result: "pointer", // Pointer to a 19-byte array
    },
    get_color_rgb: {
        parameters: ["pointer"],
        result: "pointer", // Pointer to a 3-byte array
    },
    launch_daemon: {
        parameters: [],
        result: "bool",
    },
    shutdown_daemon: {
        parameters: ["u8"],
        result: "bool",
    },
    free_device: {
        parameters: ["pointer"],
        result: "void",
    },
    free_name: {
        parameters: ["pointer"],
        result: "void",
    },
    free_color_rgb: {
        parameters: ["pointer"],
        result: "void",
    },
} as const;

const dylib = Deno.dlopen("./librustbee.so", symbols);

export const {
    new_device,
    try_connect,
    try_disconnect,
    set_power,
    set_brightness,
    get_power,
    get_brightness,
    get_name,
    get_color_rgb,
    launch_daemon,
    shutdown_daemon,
    free_device,
    free_name,
    free_color_rgb,
} = dylib.symbols;
