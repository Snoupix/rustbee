<script lang="ts">
	import { Lightbulb, BluetoothSearching } from "lucide-svelte";

    let search_input: HTMLInputElement | null = $state(null);

    function focus_search() {
        search_input?.focus();
    }
</script>

<header>
	<nav>
        <div>
            <Lightbulb size="2.5rem" /><span>Rustbee</span>
        </div>
        <!-- svelte-ignore a11y_no_static_element_interactions,a11y_click_events_have_key_events -->
        <div class="search" onclick={focus_search}>
            <span>Search:</span>
            <input type="text" bind:this={search_input}>
            <div class="bt-wrapper" title="Click to search BT devices with a (partial) name">
                <BluetoothSearching class="bt" />
            </div>
        </div>
    </nav>
</header>

<style lang="postcss">
	header {
		@apply flex flex-col justify-center fixed top-0 left-0 h-24 w-full p-10 border-b-2 border-b-extra;

		nav {
			@apply flex flex-row justify-between items-center text-4xl;

			div {
				@apply flex flex-row justify-center items-center gap-2;
			}

            .search {
                @apply flex flex-row justify-center items-center gap-4 border-2 border-solid border-primary rounded-full px-2 py-1 w-5/12 text-xl;

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
</style>
