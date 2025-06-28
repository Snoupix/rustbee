/** @type {import('tailwindcss').Config} */
export default {
	content: ["./src/**/*.{html,js,svelte,ts}"],
	theme: {
		extend: {
			colors: {
				primary: "#e7e7e4",
				secondary: "#0f0f10",
				extra: "#303036",
				contrast: "hsl(240 4% 10%)",
			},
			height: {
				header: "6rem",
				subheader: "4rem",
			},
		},
	},
	plugins: [],
};
