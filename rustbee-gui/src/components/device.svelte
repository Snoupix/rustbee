<script lang="ts">
	import { onDestroy } from "svelte";
	import { Lightbulb, Palette } from "lucide-svelte";
	import ColorPicker from "svelte-awesome-color-picker";

	import type { RgbaColor } from "svelte-awesome-color-picker";

	import { call } from "$/lib/stores/caller";
	import { rust_fn_e } from "$/lib/types";
	import { debounce } from "$/lib/utils";

	import type { Device } from "$/lib/types";

	let {
		addr: _addr,
		device,
	}: {
		addr: string;
		device: Device;
	} = $props();

	const addr = JSON.parse(_addr) as Array<number>;

	let is_colorpicker_open = $state(false);
	let { fn: color_debounce, clear_timeout: clear_color_timeout } = debounce<{ hex: string; rgba: RgbaColor }>(
		({ hex, rgba }) => update_color(hex, rgba),
		100,
	);

	onDestroy(() => {
		clear_color_timeout();
	});

	async function toggle_device_power() {
		await call(rust_fn_e.set_power, { addr, power_state: !device.power_state });
	}

	async function update_color(hex: string, rgba: RgbaColor) {
		if (device.current_color.actual_value.reduce((a, b) => a + b) == rgba.r + rgba.g + rgba.b) {
			return;
		}

		device.current_color.actual_value = [rgba.r, rgba.g, rgba.b];

		await call(rust_fn_e.set_colors, { address: addr, r: rgba.r, g: rgba.g, b: rgba.b });
	}
</script>

<div class="device">
	<div class="header">
		<!-- TODO: onclick, copy HEX addr to clipboard -->
		<div class="name">
			<Lightbulb class="mb-1" size="1.5rem" />
			<h2 title={addr.map(x => x.toString(16).toUpperCase().padStart(2, "0")).join(":")}>
				{device.name || "Unknown device name"}
			</h2>
		</div>
		<div class="status">
			<div class={device.is_connected ? "green-circle" : "red-circle"}></div>
			<span>{device.is_connected ? "Connected" : "Disconnected"}</span>
		</div>
	</div>
	<div class="spaced">
		<span>Power</span>
		<button class="button" onclick={toggle_device_power}>
			<div class={device.power_state ? "green-circle" : "red-circle"}></div>
			<span>{device.power_state ? "On" : "Off"}</span>
		</button>
	</div>
	<div class="spaced">
		<span>Brightness</span>
		<div>
			<span>{device.brightness}%</span>
			<input min={0} max={100} value={device.brightness} type="range" />
		</div>
	</div>
	<div class="spaced">
		<span>Color</span>
		<div>
			<ColorPicker
				label=""
				isOpen={is_colorpicker_open}
				--input-size="0.75rem"
				rgb={{
					r: device.current_color.actual_value[0],
					g: device.current_color.actual_value[1],
					b: device.current_color.actual_value[2],
					a: 1,
				}}
				on:input={({ detail: { hex, rgb: rgba } }) =>
					hex != undefined && rgba != undefined && color_debounce({ hex, rgba })} />
			<Palette
				class="cursor-pointer"
				color={`rgb(${device.current_color.actual_value[0]}, ${device.current_color.actual_value[1]}, ${device.current_color.actual_value[2]})`}
				onclick={() => (is_colorpicker_open = !is_colorpicker_open)} />
		</div>
	</div>
</div>

<style lang="postcss" scoped>
	.device {
		width: calc(25% - 1.8rem);
		@apply h-80 flex flex-col justify-center items-center bg-contrast border border-extra rounded-xl gap-3;

		> div {
			@apply flex flex-row justify-center items-center gap-2 px-12;
		}

		.header {
			@apply flex-col pb-12 gap-3;

			.name {
				@apply flex flex-row justify-center items-center gap-2 cursor-pointer;

				h2 {
					@apply text-xl font-medium;
				}
			}

			.status {
				@apply flex flex-row justify-center items-center gap-2;
			}
		}

		.spaced {
			@apply w-full justify-between;

			> span {
				@apply opacity-75;
			}

			> div,
			> button {
				@apply flex flex-row justify-center items-center gap-2;

				input[type="range"] {
					@apply accent-purple-800 w-32;
				}
			}
		}
	}

	.green-circle,
	.red-circle {
		@apply w-3 h-3 rounded-full;
	}

	.green-circle {
		@apply bg-emerald-400;
	}

	.red-circle {
		@apply bg-red-600;
	}
</style>
