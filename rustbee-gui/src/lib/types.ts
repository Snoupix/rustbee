export type State = {
	color: Array<number>; // length 3; R G B
	brightness: number;
	devices_found: Array<DeviceFound>;
};

export type DeviceFound = {
	address: Array<number>;
	name: String;
};

export type Devices = Map<Array<number>, Device>;
export type Device = {
	name: string;
	is_found: boolean;
	is_connected: boolean;
	power_state: boolean;
	brightness: number;
	current_color: {
		actual_value: Array<number>; // length 3; R G B
	};
};

export type LogLevel = (typeof log_level_e)[keyof typeof log_level_e];

export const log_level_e = Object.freeze({
	info: "info",
	warn: "warn",
	debug: "debug",
	error: "error",
	trace: "trace",
});

export type RustFn = (typeof rust_fn_e)[keyof typeof rust_fn_e];

export const rust_fn_e = Object.freeze({
	get_state: "get_global_state",
	get_devices: "get_devices",
	set_power: "set_power",
	set_power_all: "set_power_all",
	set_brightness: "set_brightness",
	set_brightness_all: "set_brightness_all",
	get_brightness: "get_brightness",
});
