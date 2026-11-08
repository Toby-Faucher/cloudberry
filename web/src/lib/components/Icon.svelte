<script lang="ts" module>
	const paths = {
		play: 'M8 5.5v13a1 1 0 0 0 1.5.86l10.6-6.5a1 1 0 0 0 0-1.72L9.5 4.64A1 1 0 0 0 8 5.5Z',
		pause: 'M7 5h3.5v14H7zM13.5 5H17v14h-3.5z',
		skip: 'M5 6.2v11.6a1 1 0 0 0 1.55.83l8.4-5.8a1 1 0 0 0 0-1.66l-8.4-5.8A1 1 0 0 0 5 6.2ZM17 5h2.5v14H17z',
		up: 'm6 15 6-6 6 6',
		down: 'm6 9 6 6 6-6',
		x: 'M6 6l12 12M18 6 6 18',
		plus: 'M12 5v14M5 12h14',
		grip: 'M9 6h.01M15 6h.01M9 12h.01M15 12h.01M9 18h.01M15 18h.01',
		check: 'm5 12.5 4.5 4.5L19 7.5',
		alert: 'M12 8v5M12 16.5h.01',
		volume: 'M4 9.5h3l4.5-4v13L7 14.5H4zM15.5 9a4 4 0 0 1 0 6M18 6.5a7.5 7.5 0 0 1 0 11'
	} as const;
	export type IconName = keyof typeof paths;
</script>

<script lang="ts">
	let { name, size = 18 }: { name: IconName; size?: number } = $props();
	const filled = $derived(name === 'play' || name === 'pause' || name === 'skip');
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 24 24"
	aria-hidden="true"
	fill={filled ? 'currentColor' : 'none'}
	stroke={filled ? 'none' : 'currentColor'}
	stroke-width={name === 'grip' ? 3 : 2}
	stroke-linecap="round"
	stroke-linejoin="round"
>
	<path d={paths[name]} />
</svg>
