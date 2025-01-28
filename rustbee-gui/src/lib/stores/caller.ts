import { derived, writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";

import type { InvokeArgs } from "@tauri-apps/api/core";

import { log_level_e, rust_fn_e } from "$/lib/types";

import type { RustFn, LogLevel, State, Devices, DevicesPayload } from "$/lib/types";

export const current_call_count = writable(0);
export const is_loading = derived(current_call_count, $count => $count > 0);
export const error = writable<string | null>(null);
export const app_state = writable<State | null>(null);
export const devices = writable<Devices | null>(null);

function update_curr_call_count(increment: boolean) {
	current_call_count.update(count => (increment ? (count += 1) : (count -= 1)));
}

export async function log(data: string, log_level: LogLevel) {
	console.log(`[${log_level}]`, data);
	await invoke("log", { data, log_level });
}

export async function call<T>(fn: RustFn, args?: InvokeArgs): Promise<T> {
	update_curr_call_count(true);

	return (await invoke(fn, args)
		.catch(async (err: unknown) => {
			error.set(err as string);
			await log(err as string, log_level_e.error);
		})
		.finally(() => update_curr_call_count(false))) as T;
}

export async function update_devices(payload?: DevicesPayload | undefined) {
	const _devices = Object.entries(payload ?? (await call<DevicesPayload>(rust_fn_e.update_devices)));

	_devices.forEach(([k, v]) => {
		devices.update(data => {
			let map = data;
			if (map == null) {
				map = new Map();
			}

			map.set(k, v);

			return map;
		});
	});
}

export async function fetch_initial_state() {
	app_state.set(await call(rust_fn_e.fetch_init_state));
}

export async function update_app_state() {
	app_state.set(await call(rust_fn_e.get_state));
}
