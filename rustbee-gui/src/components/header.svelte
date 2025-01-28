<script lang="ts">
	import { onDestroy, onMount } from "svelte";
	import { fade } from "svelte/transition";
	import { Lightbulb, BluetoothSearching } from "lucide-svelte";

	import { is_loading } from "$/lib/stores/caller";
	import { debounce } from "$/lib/utils";

	let search_input: HTMLInputElement | null = $state(null);
	let gradient_idx = $state(0);

	const gradients: Record<string, string> = {
		rainbow: `linear-gradient(
          to right,
          #FF0080,
          #FF00FF,
          #00FFFF,
          #FF0080,
          #FF00FF,
          #00FFFF,
          #FF0080
        )`,
		sunset: `linear-gradient(
          to right,
          #FF512F,
          #FF9671,
          #FFC75F,
          #FF512F,
          #FF9671,
          #FFC75F,
          #FF512F
        )`,
		ocean: `linear-gradient(
          to right,
          #0083B0,
          #00B4DB,
          #00F2FE,
          #0083B0,
          #00B4DB,
          #00F2FE,
          #0083B0
        )`,
		neon: `linear-gradient(
          to right,
          #FF1493,
          #00FF00,
          #00FFFF,
          #FF1493,
          #00FF00,
          #00FFFF,
          #FF1493
        )`,
		cyberpunk: `linear-gradient(
          to right,
          #FF00FF,
          #00FFFF,
          #FF00FF,
          #00FFFF,
          #FF00FF
        )`,
		synthwave: `linear-gradient(
          to right,
          #FF00FF,
          #00FFFF,
          #FF00FF,
          #00FFFF,
          #FF00FF,
          #00FFFF,
          #FF00FF
        )`,
		neon_fire: `linear-gradient(
          to right,
          #FF0000,
          #FF00FF,
          #FFFF00,
          #FF0000,
          #FF00FF,
          #FFFF00,
          #FF0000
        )`,
		electric_blue: `linear-gradient(
          to right,
          #00FFFF,
          #0099FF,
          #00FF00,
          #00FFFF,
          #0099FF,
          #00FF00,
          #00FFFF
        )`,
		plasma: `linear-gradient(
          to right,
          #FF1493,
          #FF00FF,
          #00FFFF,
          #FF1493,
          #FF00FF,
          #00FFFF,
          #FF1493
        )`,
		candy: `linear-gradient(
          to right,
          #FF0080,
          #FF00FF,
          #00FFFF,
          #FF0080,
          #FF00FF,
          #00FFFF,
          #FF0080
        )`,
		toxic: `linear-gradient(
          to right,
          #00FF00,
          #FFFF00,
          #00FF99,
          #00FF00,
          #FFFF00,
          #00FF99,
          #00FF00
        )`,
		ultraviolet: `linear-gradient(
          to right,
          #9933FF,
          #FF00FF,
          #CC00FF,
          #9933FF,
          #FF00FF,
          #CC00FF,
          #9933FF
        )`,
		solar_flare: `linear-gradient(
          to right,
          #FF4400,
          #FFFF00,
          #FF8800,
          #FF4400,
          #FFFF00,
          #FF8800,
          #FF4400
        )`,
		neon_dream: `linear-gradient(
          to right,
          #FF0099,
          #00FFFF,
          #FF00FF,
          #FF0099,
          #00FFFF,
          #FF00FF,
          #FF0099
        )`,
	} as const;

	const gradients_names = Object.keys(gradients);

	const { fn: shuffle_debounce, clear_timeout: clear_debounce } = debounce(shuffle_gradient_index, 2250);

	onMount(() => {
		shuffle_gradient_index();

		is_loading.subscribe(bool => !bool && shuffle_debounce(undefined));
	});

	onDestroy(clear_debounce);

	function shuffle_gradient_index() {
		const old = gradient_idx;
		const get_new = () => Math.floor(Math.random() * gradients_names.length);
		gradient_idx = get_new();

		while (gradient_idx == old) {
			gradient_idx = get_new();
		}
	}

	function focus_search() {
		search_input?.focus();
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
			<input type="text" bind:this={search_input} />
			<div class="bt-wrapper" title="Click to search BT devices with a (partial) name">
				<BluetoothSearching class="bt" />
			</div>
		</div>
	</nav>
</header>
{#if $is_loading}
	<div transition:fade class="rainbow-loader" style="--gradient: {gradients[gradients_names[gradient_idx]]};"></div>
{/if}

<style lang="postcss">
	header {
		@apply flex flex-col justify-center h-header w-full p-10 border-b-2 border-b-extra;

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
