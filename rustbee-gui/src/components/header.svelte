<script lang="ts">
	import { onDestroy, onMount } from "svelte";
	import { fade } from "svelte/transition";
	import { event } from "@tauri-apps/api";
	import { Lightbulb, BluetoothSearching } from "lucide-svelte";

	import { app_state, call, is_loading } from "$/lib/stores/caller";
	import { rust_fn_e } from "$/lib/types";
	import { debounce } from "$/lib/utils";
	import _gradients from "$/lib/gradients";

	import type { DeviceFound } from "$lib/types";

	// TODO: Move device stream logic to a child component and give it search_input props

	let is_searching = $state(false);
	let is_stream_finished = $state(false);
	let search_input_ref: HTMLInputElement | null = $state(null);
	let search_input_value = $state("");
	let stream_unlisten_evt: (() => void) | null = $state(null);
	let end_stream_unlisten_evt: (() => void) | null = $state(null);
	let gradient_idx = $state(0);

	const gradients: Record<string, string> = _gradients;

	const gradients_names = Object.keys(gradients);

	const { fn: shuffle_debounce, clear_timeout: clear_debounce } = debounce(shuffle_gradient_index, 2250);

	onMount(() => {
		shuffle_gradient_index();

		is_loading.subscribe(bool => !bool && shuffle_debounce(undefined));
	});

	onDestroy(() => {
		clear_debounce();

		stream_unlisten_evt?.();
		end_stream_unlisten_evt?.();
	});

	$inspect(search_input_value);

	function shuffle_gradient_index() {
		const old = gradient_idx;
		const get_new = () => Math.floor(Math.random() * gradients_names.length);
		gradient_idx = get_new();

		while (gradient_idx == old) {
			gradient_idx = get_new();
		}
	}

	function focus_search() {
		search_input_ref?.focus();
	}

	function close_device_list() {
		is_searching = false;
		is_stream_finished = false;

        app_state.update(state => {
            if (state == null) return state;

            state.devices_found = [];
            return state;
        });
	}

	async function search_devices() {
		if (is_searching) return;

		is_searching = true;

		const id = await call(rust_fn_e.start_bt_stream, { name: search_input_value });

		stream_unlisten_evt = await event.listen(`bt_stream_${id}_data`, event => {
			const device = event.payload as DeviceFound;

			app_state.update(state => {
                const addr_count = device.address.reduce((a, b) => a + b);
				if (
					state?.devices_found.some(
						d => d.address.reduce((a, b) => a + b) == addr_count
					)
				) {
					return state;
				}

				state?.devices_found.push(device);
				return state;
			});
		});

		end_stream_unlisten_evt = await event.listen(`bt_stream_${id}_end`, () => {
			console.log("end stream");
			is_stream_finished = true;
			stream_unlisten_evt?.();
		});
	}

    function add_device(address: Array<number>) {
        // TODO
    }
</script>

<header>
	<nav>
		<div>
			<Lightbulb class="mb-2" size="2.5rem" />
			<h1>Rustbee</h1>
		</div>
		<!-- svelte-ignore a11y_no_static_element_interactions,a11y_click_events_have_key_events -->
		<div class="search" onclick={focus_search}>
			<span>Search:</span>
			<input type="text" bind:this={search_input_ref} bind:value={search_input_value} disabled={is_searching} />
			<div class="bt-wrapper" title="Click to search BT devices with a (partial) name" onclick={search_devices}>
				<BluetoothSearching class="bt" />
			</div>
		</div>
	</nav>
	{#if is_searching && $app_state != null}
		{#if is_stream_finished}
			<button class="close_btn" onclick={close_device_list}>X</button>
		{/if}
		<div class="search_section">
			{#key $app_state}
				{#each $app_state.devices_found as device}
					<button
						onclick={() => add_device(device.address)}
						>{device.name} - {device.address
							.map(x => x.toString(16).toUpperCase().padStart(2, "0"))
							.join(":")}</button>
				{/each}
			{/key}
			{#if !is_stream_finished}
				<!-- TODO: Spinner -->
				<span>Loading...</span>
			{/if}
		</div>
	{/if}
</header>
{#if $is_loading}
	<div transition:fade class="rainbow-loader" style="--gradient: {gradients[gradients_names[gradient_idx]]};"></div>
{/if}

<style lang="postcss">
	header {
		@apply relative flex flex-col justify-center h-header w-full p-10 border-b-2 border-b-extra;

		nav {
			@apply flex flex-row justify-between items-center text-4xl;

			div {
				@apply flex flex-row justify-center items-center gap-2 pt-2;

				h1 {
					@apply font-medium;
				}
			}

			.search {
				@apply flex flex-row justify-center items-center gap-4 border-2 border-solid border-primary rounded-full px-2 py-1 w-2/12 text-xl;

				input {
					@apply w-5/12 text-primary bg-secondary outline-none;
				}

				.bt-wrapper {
					@apply bg-primary border-2 border-solid border-primary rounded-full p-1 transition-all duration-200;

					:global(.bt) {
						@apply text-secondary;
					}

					&:hover {
						@apply bg-secondary cursor-pointer;

						:global(.bt) {
							@apply text-primary;
						}
					}
				}
			}
		}

		.close_btn {
			@apply absolute top-0 right-0 w-12 h-12;
		}

		.search_section {
			@apply w-full h-auto max-h-[20vh] flex flex-col justify-center items-center flex-wrap;
		}
	}

	.rainbow-loader {
		@apply absolute top-24 left-0 h-[2px] w-full overflow-hidden;
	}

	.rainbow-loader::after {
		content: "";
		position: absolute;
		top: 0;
		left: 0;
		height: 100%;
		width: 100%;
		background: var(--gradient);
		background-size: 200% 100%;
		animation: rainbow-loading 2.25s linear infinite;
	}

	@keyframes rainbow-loading {
		0% {
			background-position: 100% 0;
		}
		100% {
			background-position: -100% 0;
		}
	}
</style>
