import * as C from "./ffi.ts";

const ADDRS = [
	new Uint8Array([0xE8, 0xD4, 0xEA, 0xC4, 0x62, 0x00], 0, 6),
	new Uint8Array([0xEC, 0x27, 0xA7, 0xD6, 0x5A, 0x9C], 0, 6),
];

if (!C.launch_daemon()) {
    console.error("Failed to launch rustbee daemon");
    Deno.exit(1);
}

for (const addr of ADDRS) {
    const device_ptr = C.new_device(Deno.UnsafePointer.of(addr));

    if (!C.try_connect(device_ptr)) {
        console.error("Failed to connect to the device.");
    }
    if (!C.set_power(device_ptr, parseInt(Deno.args[0] || "1"))) {
        console.error("Failed to set device power");
    }

    C.free_device(device_ptr);
}
