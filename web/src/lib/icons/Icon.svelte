<script lang="ts" module>
	export type IconName =
		| 'play'
		| 'pause'
		| 'skip'
		| 'plus'
		| 'chat'
		| 'send'
		| 'expand'
		| 'shrink'
		| 'volume'
		| 'check'
		| 'close'
		| 'mic'
		| 'bell'
		| 'bell-off'
		| 'crown'
		| 'down'
		| 'up'
		| 'chev-up'
		| 'chev-down'
		| 'grip'
		| 'alert'
		| 'list'
		| 'list-plus'
		| 'keyboard'
		| 'poke'
		| 'undo'
		| 'pencil'
		| 'back'
		| 'shuffle'
		| 'trash'
		// Reactions (ids from reactions.ts).
		| 'flame'
		| 'laugh'
		| 'heart'
		| 'clap'
		| 'party'
		| 'zzz';
</script>

<script lang="ts">
	/**
	 * Cloudberry's pictograms: one 1.75 stroke on a 24 grid, square terminals and mitred joins,
	 * like the legends printed on a device. Transport glyphs are solid so they read on a key at a
	 * glance. `currentColor`; decorative: the control carries the label.
	 */
	let { name, size = 20 }: { name: IconName; size?: number } = $props();
</script>

<svg
	class="cb-icon"
	viewBox="0 0 24 24"
	width={size}
	height={size}
	fill="none"
	stroke="currentColor"
	stroke-width="1.75"
	stroke-linejoin="miter"
	stroke-linecap="square"
	aria-hidden="true"
>
	{#if name === 'play'}
		<path d="M7 5v14l11.5-7Z" fill="currentColor" />
	{:else if name === 'pause'}
		<path d="M6.5 5h3.5v14H6.5ZM14 5h3.5v14H14Z" fill="currentColor" stroke="none" />
	{:else if name === 'skip'}
		<path d="M5 5.5v13l9.5-6.5Z" fill="currentColor" />
		<path d="M16.5 5h2.75v14H16.5Z" fill="currentColor" stroke="none" />
	{:else if name === 'plus'}
		<path d="M12 5v14M5 12h14" />
	{:else if name === 'chat'}
		<path d="M4 5h16v11h-9l-4.5 3.5V16H4Z" />
	{:else if name === 'send'}
		<path d="M5 12h13M13 6.5l5.5 5.5-5.5 5.5" />
	{:else if name === 'expand'}
		<path d="M4 9V4h5M15 4h5v5M20 15v5h-5M9 20H4v-5" />
	{:else if name === 'shrink'}
		<path d="M9 4v5H4M20 9h-5V4M15 20v-5h5M4 15h5v5" />
	{:else if name === 'volume'}
		<path d="M4 9.5h3.5L12 6v12l-4.5-3.5H4Z" />
		<path d="M15.5 9.5v5M18.5 7.5v9" />
	{:else if name === 'check'}
		<path d="M5 12.5 9.5 17 19 7.5" />
	{:else if name === 'close'}
		<path d="M6.5 6.5l11 11M17.5 6.5l-11 11" />
	{:else if name === 'mic'}
		<path d="M9 4h6v10H9Z" />
		<path d="M6 11v1a6 6 0 0 0 12 0v-1M12 18v2.5" />
	{:else if name === 'bell' || name === 'bell-off'}
		<path d="M6.5 16V10.5a5.5 5.5 0 0 1 11 0V16l1.5 1.5H5Z" />
		<path d="M10 20.5h4" />
		{#if name === 'bell-off'}<path d="M4.5 4.5l15 15" />{/if}
	{:else if name === 'crown'}
		<path d="M4 8l4.5 4L12 5.5 15.5 12 20 8l-1.5 10h-13Z" fill="currentColor" />
	{:else if name === 'down'}
		<path d="M12 4.5v14M6.5 13l5.5 5.5 5.5-5.5" />
	{:else if name === 'up'}
		<path d="M12 19.5v-14M6.5 11 12 5.5l5.5 5.5" />
	{:else if name === 'chev-up'}
		<path d="M6.5 14.5 12 9l5.5 5.5" />
	{:else if name === 'chev-down'}
		<path d="M6.5 9.5 12 15l5.5-5.5" />
	{:else if name === 'grip'}
		<path d="M9 6h1.5v1.5H9ZM13.5 6H15v1.5h-1.5ZM9 11.25h1.5v1.5H9ZM13.5 11.25H15v1.5h-1.5ZM9 16.5h1.5V18H9ZM13.5 16.5H15V18h-1.5Z" fill="currentColor" stroke="none" />
	{:else if name === 'alert'}
		<path d="M12 4 20.5 19h-17Z" />
		<path d="M12 10v4M12 16.5v.5" />
	{:else if name === 'list'}
		<path d="M4 6h12M4 11h12M4 16h7" />
		<path d="M18.5 18.5V10l2 1" />
		<path d="M15 16.5h3.5v3.5H15Z" fill="currentColor" />
	{:else if name === 'list-plus'}
		<path d="M4 6h12M4 11h12M4 16h7M17.5 14v6M14.5 17h6" />
	{:else if name === 'keyboard'}
		<path d="M3 6.5h18v11H3Z" />
		<path d="M7 10h.5M11.75 10h.5M16.5 10h.5M8 14h8" />
	{:else if name === 'poke'}
		<path d="M10 10h4v4h-4Z" fill="currentColor" />
		<path d="M12 3.5v3M12 17.5v3M3.5 12h3M17.5 12h3M6 6l1.5 1.5M18 6l-1.5 1.5M6 18l1.5-1.5M18 18l-1.5-1.5" />
	{:else if name === 'undo'}
		<path d="M8.5 5.5 4.5 9.5l4 4" />
		<path d="M5 9.5h9.5a4.5 4.5 0 0 1 0 9H11" />
	{:else if name === 'pencil'}
		<path d="M15 4.5 19.5 9 9 19.5H4.5V15Z" />
		<path d="M12.5 7l4.5 4.5" />
	{:else if name === 'back'}
		<path d="M14.5 6.5 9 12l5.5 5.5" />
	{:else if name === 'shuffle'}
		<path d="M4 7.5h3.5l9 9H20M4 16.5h3.5l2.5-2.5M14 10l2.5-2.5H20" />
		<path d="M17.5 5 20 7.5 17.5 10M17.5 14 20 16.5 17.5 19" />
	{:else if name === 'trash'}
		<path d="M4.5 7h15M9.5 7V4.5h5V7M6.5 7l1 12.5h9l1-12.5M10.5 10.5v6M13.5 10.5v6" />
	{:else if name === 'flame'}
		<path d="M12 3c.5 3.5 6 5.5 6 11a6 6 0 0 1-12 0c0-3 1.5-4.5 3-6 .3 2 1 3 2.2 3.4C11 8.5 10.8 6 12 3Z" />
	{:else if name === 'laugh'}
		<circle cx="12" cy="12" r="8.5" />
		<path d="M7.75 10.25 9.5 8.5l1.75 1.75M12.75 10.25 14.5 8.5l1.75 1.75" />
		<path d="M8 13.5h8a4 4 0 0 1-8 0Z" />
	{:else if name === 'heart'}
		<path d="M12 19.5 4.8 12.3A4.2 4.2 0 0 1 12 7a4.2 4.2 0 0 1 7.2 5.3Z" />
	{:else if name === 'clap'}
		<path d="M5.5 10.5 9 9.5l3 10-3.5 1Z" />
		<path d="M12.5 8.5l3.5-.5 1.5 10.5-3.5.5" />
		<path d="M7 4.5l1 2.5M11.5 3.5v2.5M16 4.5l-1 2.5" />
	{:else if name === 'party'}
		<path d="M4.5 19.5 9 8l7 7Z" />
		<path d="M14 4v2.5M20 10h-2.5M18 6l-1.75 1.75M11.5 5.5l.5 1" />
	{:else if name === 'zzz'}
		<path d="M4 8h5.5L4 15h5.5M12.5 12H17l-4.5 5.5H17M16.5 3.5h4l-4 4.5h4" />
	{/if}
</svg>

<style>
	.cb-icon {
		display: block;
		flex: none;
		overflow: visible;
	}
</style>
