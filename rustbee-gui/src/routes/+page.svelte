<script lang="ts">
	import { invoke, type InvokeArgs } from "@tauri-apps/api/core";
	import { onMount } from "svelte";
	import ColorPicker from "svelte-awesome-color-picker";

	import Header from "$/components/header.svelte";

	import { rust_fn_e, log_level_e } from "$/lib/types";
	import type { LogLevel, RustFn, Device, Devices, State } from "$/lib/types";

	let power = $state(false);
	let loading = $state(false);
	let error = $state("");
	let state: State | null = $state(null);
	let devices: Devices | null = $state(null);

	onMount(async () => {
		state = await call(rust_fn_e.get_state);

		const _devices = new Map(Object.entries(await call<{ [s: string]: Device }>(rust_fn_e.get_devices)));
		_devices.forEach((v, k) => {
			if (devices == null) {
				devices = new Map();
			}

			devices.set(JSON.parse(k), v);
		});

		console.log(devices);
	});

	async function log(data: string, log_level: LogLevel) {
		console.log(log_level, data);
		await invoke("log", { data, log_level });
	}

	async function call<T>(fn: RustFn, args?: InvokeArgs): Promise<T> {
		loading = true;

		return (await invoke(fn, args)
			.catch(async err => {
                error = err;
                await log(err, log_level_e.error)
            })
			.finally(() => (loading = false))) as T;
	}

	async function set_power(addr: Array<number>, power_state: boolean) {
		error = await invoke("set_power", { addr, power_state });
	}

	async function set_brightness_all(brightness: number) {
		error = await invoke("set_brightness_all", { brightness });
	}

	async function set_brightness(addr: Array<number>, brightness: number) {
		error = await invoke("set_brightness", { addr, brightness });
	}
</script>

<main>
	<Header />
	{#if loading}
		<div>loading...</div>
	{/if}
	<button
		onclick={async () => {
			await call(rust_fn_e.set_power_all, { power_state: power });
			power = !power;
		}}>Toggle power all</button>
	<input type="range" min={0} max={100} onchange={e => set_brightness_all(parseInt(e.currentTarget.value))} />
	{#each devices! as [addr, device] (addr)}
		<div>
			<h2>
				{device.name || "Device name unknown"} - {addr
					.map(x => x.toString(16).toUpperCase().padStart(2, "0"))
					.join(":")}
			</h2>
			<span>{device.power_state}</span>
			<span>{device.is_found}</span>
			<span>{device.brightness}</span>
			<span>{device.is_connected}</span>
			<span>{device.current_color.actual_value}</span>
			<ColorPicker />
		</div>
	{/each}
	<!-- <form class="row" onsubmit={greet}>
    <input id="greet-input" placeholder="Enter a name..." bind:value={name} />
    <button type="submit">Greet</button>
  </form> -->
	<p>{error}</p>
</main>

<style lang="postcss">
	main {
		@apply flex flex-col justify-center items-center text-center w-screen h-screen pt-[10vh] bg-secondary text-primary;

		input[type="range"] {
			@apply w-[50%];
		}
	}

	button {
		@apply text-primary border border-solid border-primary rounded-lg px-[2vw] py-[1vh] bg-secondary bg-opacity-25 cursor-pointer;

		&:hover {
			border-color: var(--bg-color);
			background-color: var(--color);
			color: var(--bg-color);
		}
	}
</style>
