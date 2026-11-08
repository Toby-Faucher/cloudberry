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
		| 'down';
</script>

<script lang="ts">
	/**
	 * Cloudberry's own chunky icons: filled shapes with fat rounded joins so they sit with
	 * Fredoka's weight. 24-unit grid; `currentColor`. Decorative: the button carries the label.
	 */
	let { name, size = 22 }: { name: IconName; size?: number } = $props();
</script>

<svg
	class="cb-icon"
	viewBox="0 0 24 24"
	width={size}
	height={size}
	fill="currentColor"
	stroke="currentColor"
	stroke-linejoin="round"
	stroke-linecap="round"
	aria-hidden="true"
>
	{#if name === 'play'}
		<path d="M7.5 5 19 12 7.5 19Z" stroke-width="3.4" />
	{:else if name === 'pause'}
		<rect x="5" y="4" width="5" height="16" rx="2" stroke="none" />
		<rect x="14" y="4" width="5" height="16" rx="2" stroke="none" />
	{:else if name === 'skip'}
		<path d="M5 6 13.5 12 5 18Z" stroke-width="3.4" />
		<rect x="15.5" y="4.5" width="4.5" height="15" rx="2.25" stroke="none" />
	{:else if name === 'plus'}
		<path d="M12 4.5v15M4.5 12h15" fill="none" stroke-width="4.4" />
	{:else if name === 'chat'}
		<path d="M6.5 4h11A3.5 3.5 0 0 1 21 7.5v6a3.5 3.5 0 0 1-3.5 3.5h-6L6 20.5V17h.5A3.5 3.5 0 0 1 3 13.5v-6A3.5 3.5 0 0 1 6.5 4Z" stroke-width="1.6" />
	{:else if name === 'send'}
		<path d="M4 11.2 19.5 4.5 14 20l-2.6-6.2Z" stroke-width="3" />
	{:else if name === 'expand'}
		<path d="M4.5 9.5v-5h5M14.5 4.5h5v5M19.5 14.5v5h-5M9.5 19.5h-5v-5" fill="none" stroke-width="3.2" />
	{:else if name === 'shrink'}
		<path d="M9.5 4.5v5h-5M19.5 9.5h-5v-5M14.5 19.5v-5h5M4.5 14.5h5v5" fill="none" stroke-width="3.2" />
	{:else if name === 'volume'}
		<path d="M4 9.5h3.5L12 5.5v13l-4.5-4H4Z" stroke-width="2.6" />
		<path d="M15.5 9a4 4 0 0 1 0 6M18.5 6.5a7.5 7.5 0 0 1 0 11" fill="none" stroke-width="2.6" />
	{:else if name === 'check'}
		<path d="M5 12.5 9.8 17 19 7" fill="none" stroke-width="3.6" />
	{:else if name === 'close'}
		<path d="M6.5 6.5l11 11M17.5 6.5l-11 11" fill="none" stroke-width="3.6" />
	{:else if name === 'mic'}
		<rect x="8.5" y="3" width="7" height="11.5" rx="3.5" stroke="none" />
		<path d="M5.5 11.5a6.5 6.5 0 0 0 13 0M12 18v3" fill="none" stroke-width="2.6" />
	{:else if name === 'bell' || name === 'bell-off'}
		<path d="M12 3.5a5.5 5.5 0 0 0-5.5 5.5v3.5L4.5 16h15l-2-3.5V9A5.5 5.5 0 0 0 12 3.5Z" stroke-width="2" />
		<circle cx="12" cy="19.2" r="2" stroke="none" />
		{#if name === 'bell-off'}<path d="M4 4l16 16" fill="none" stroke-width="3" class="slash" />{/if}
	{:else if name === 'crown'}
		<path d="M4 8.5 8.2 12 12 5.5 15.8 12 20 8.5 18.5 18h-13Z" stroke-width="2.4" />
	{:else if name === 'down'}
		<path d="M12 4.5v12M6.5 12 12 17.5 17.5 12" fill="none" stroke-width="3.4" />
	{/if}
</svg>

<style>
	.cb-icon {
		display: block;
		flex: none;
		overflow: visible;
	}
</style>
