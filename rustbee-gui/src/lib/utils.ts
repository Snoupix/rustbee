// Where T is [K, V]
export function map_iter_to_array<T>(iter: MapIterator<T>): Array<T> {
	const array: Array<T> = [];

	for (const val of iter) {
		array.push(val);
	}

	return array;
}

export function debounce<T>(closure: (args: T) => void | Promise<void>, delay: number = 300) {
	let timeout: number | null = null;
	const clear = (t: number | null) => {
		if (t == null) return;

		clearTimeout(t);
	};

	// Cannot return a "tuple" bc in JS it is an actual array
	// and the weird JS thing it does is inferring closure args
	// from others even if, let's say, one had no arguments.
	return {
		fn: (args: T) => {
			clear(timeout);

			timeout = setTimeout(() => closure(args), delay);
		},
		clear_timeout: (_: void) => clear(timeout),
	};
}
