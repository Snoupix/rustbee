<script lang="ts">
	import { onDestroy, onMount } from "svelte";
	import { event } from "@tauri-apps/api";

	import type { UnlistenFn } from "@tauri-apps/api/event";

	import Header from "$/components/header.svelte";
	import Subheader from "$/components/subheader.svelte";
	import Device from "$/components/device.svelte";
	import { call, error, devices, update_devices, fetch_initial_state, log } from "$/lib/stores/caller";
	import { log_level_e, rust_fn_e } from "$/lib/types";

	import type { DevicesPayload } from "$/lib/types";

	let state_unlisten: UnlistenFn | null = $state(null);
	let is_browser = $state(false);

	onMount(async () => {
		// @ts-ignore window isn't typed but nvm
		if (!window?.__TAURI_INTERNALS__) {
			is_browser = true;
			return;
		}

		await fetch_initial_state();
		await update_devices();

		state_unlisten = await event.listen("device_sync", async event => {
			await log(JSON.stringify(event.payload), log_level_e.info);
			await update_devices(event.payload as DevicesPayload);
		});
	});

	onDestroy(() => {
		if (state_unlisten != null) {
			state_unlisten();
		}
	});

	async function set_power(addr: Array<number>, power_state: boolean) {
		await call(rust_fn_e.set_power, { addr, power_state });
	}

	async function set_brightness_all(brightness: number) {
		await call(rust_fn_e.set_brightness_all, { brightness });
	}

	async function set_brightness(addr: Array<number>, brightness: number) {
		await call(rust_fn_e.set_brightness, { addr, brightness });
	}
</script>

<main>
	{#if is_browser}
		<section class="is-browser">
			<h1>Sorry, you can only use Rustbee on the GUI and not your browser (yet ?).</h1>
		</section>
	{:else}
		<Header />
		<Subheader />

		<section class="main">
			<div class="device-wrapper">
				{#if $devices != null && $devices!.size > 0}
					{#each $devices as [addr, device] (addr)}
						<Device {addr} {device} />
					{/each}
				{/if}
			</div>

			{#if $error != null}
				<button onclick={() => ($error = null)}>clear error message</button>
				<h1>{$error}</h1>
			{/if}
		</section>
	{/if}
</main>

<style lang="postcss">
	main {
		@apply w-screen h-screen bg-secondary text-primary;

		.is-browser {
			@apply pb-44 w-screen h-screen flex justify-center items-center text-center;
		}

		.main {
			@apply flex flex-col justify-center items-center text-center w-full bg-secondary text-primary overflow-y-scroll;

			height: calc(100% - (theme("height.header") + theme("height.subheader")));

			.device-wrapper {
				@apply flex flex-row flex-wrap gap-8 w-full h-full p-8;
			}
		}
	}
</style>
