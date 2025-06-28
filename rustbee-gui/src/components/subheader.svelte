<script lang="ts">
	import { onDestroy, onMount } from "svelte";
	import { Palette } from "lucide-svelte";
	import ColorPicker, { type RgbaColor } from "svelte-awesome-color-picker";

	import { call, devices, update_devices } from "$/lib/stores/caller";
	import { rust_fn_e } from "$/lib/types";
	import { debounce, map_iter_to_array } from "$/lib/utils";

	import type { Devices } from "$/lib/types";

	let is_ready = $state(false);
	let setup_timeout: number | null = $state(null);
	let is_clicked = $state(false);
	let is_colorpicker_open = $state(false);
	let are_lights_on = $state(true);
	let current_color = $state([255, 255, 255]);
	let brightness = $state(100);

	const { fn: brightness_debounce, clear_timeout: clear_brightness_debounce } = debounce(update_brightnesses, 100);
	const { fn: colors_debounce, clear_timeout: clear_colors_debounce } = debounce<{ hex: string; rgba: RgbaColor }>(
		({ hex, rgba }) => update_colors(hex, rgba),
		100,
	);

	onMount(() => {
		// This stupid trick is needed to avoid the "oninput" trigger on the ColorPicker setup
		setup_timeout = setTimeout(() => (is_ready = true), 500);

		init($devices);

		// This will "recalculate" if the power all btn should be ON or OFF
		devices.subscribe(init);
	});

	onDestroy(() => {
		clear_brightness_debounce();
		clear_colors_debounce();

		if (setup_timeout != null) clearTimeout(setup_timeout);
	});

	function init(devices: Devices | null) {
		if (devices == null) {
			return;
		}

		const entries = devices.entries();

		// true by default and if ALL are ON, false
		are_lights_on =
			// This, is a trick because Iterator.toArray or Iterator.map fn are not impl by every browser
			(entries.map != undefined ? entries : map_iter_to_array(entries))
				.map(([_addr, device]) => device.power_state)
				.reduce((acc, is_on) => {
					if (!acc) {
						return acc;
					}

					return is_on;
				}, true);
	}

	async function update_colors(hex: string, rgba: RgbaColor) {
		if (!is_ready) return;

		current_color = [rgba.r, rgba.g, rgba.b];

		await call(rust_fn_e.set_colors_all, { r: rgba.r, g: rgba.g, b: rgba.b });
	}

	async function toggle_lights() {
		are_lights_on = !are_lights_on;
		await call(rust_fn_e.set_power_all, { power_state: are_lights_on });
		await update_devices();
	}

	async function update_brightnesses() {
		await call(rust_fn_e.set_brightness_all, { brightness });

		if ($devices == null) {
			update_devices();
			return;
		}

		// This is a manual update but do we really want
		// shallow data to avoid a state fetch ?
		let new_devices = new Map($devices);
		for (const [addr, device] of $devices) {
			new_devices.set(addr, { ...device, brightness });
		}

		devices.set(new_devices);
	}
</script>

<section>
	<!-- TODO: Impl those anims -->
	<!-- TODO: Fix styling, center color-picker -->
	<button
		class={[is_clicked && are_lights_on && "green-anim", is_clicked && !are_lights_on && "red-anim"]}
		onmousedown={() => (is_clicked = true)}
		onmouseup={() => (is_clicked = false)}
		onclick={toggle_lights}>Turn {are_lights_on ? "OFF" : "ON"} all</button>
	<div class="color-picker">
		<Palette
			class="cursor-pointer"
			color={`rgb(${current_color[0]}, ${current_color[1]}, ${current_color[2]})`}
			onclick={() => (is_colorpicker_open = !is_colorpicker_open)} />
		<ColorPicker
			label=""
			isOpen={is_colorpicker_open}
			--input-size="0.75rem"
			rgb={{
				r: current_color[0],
				g: current_color[1],
				b: current_color[2],
				a: 1,
			}}
			on:input={({ detail: { hex, rgb: rgba } }) =>
				hex != undefined && rgba != undefined && colors_debounce({ hex, rgba })} />
	</div>
	<input
		type="range"
		min={0}
		max={100}
		onchange={e => {
			brightness = parseInt(e.currentTarget.value);
			brightness_debounce(undefined);
		}} />
</section>

<style scoped lang="postcss">
	section {
		@apply h-subheader px-12 flex flex-row justify-between items-center border-b-2 border-extra;

		button {
			@apply outline-none px-4 py-2 rounded-xl border border-extra hover:border-primary hover:shadow-extra hover:shadow-xl transition-all duration-300;
		}

		> div {
			@apply m-auto h-full;
		}

		.color-picker {
			@apply flex flex-row-reverse justify-center items-center gap-2;
		}

		input[type="range"] {
			@apply w-2/12 accent-purple-800;
		}
	}
</style>
