<script lang="ts">
	import Icon from '#lib/icons/Icon.svelte';
	import { initials } from '#lib/util.ts';
	import { ui } from '#lib/ui.svelte.ts';

	/** Decorative by default: the name is almost always printed next to it. `crown`: show the DJ-of-the-day
	 * crown if this is them (only the header listener stack asks, so rows stay calm). */
	let { name, size = 32, me = false, crown = false }: { name: string; size?: number; me?: boolean; crown?: boolean } = $props();
	const dj = $derived(crown && ui.dj !== null && ui.dj === name);
</script>

<span class="avatar" class:me class:small={size < 24} style:--s="{size}px" aria-hidden="true"
	>{initials(name)}{#if dj}<span class="crown"><Icon name="crown" size={size < 24 ? 11 : Math.round(size * 0.52)} /></span>{/if}</span
>

<style>
	/* A small printed tile: mono initials, monochrome. You get an orange lamp in the corner. */
	.avatar {
		position: relative;
		display: inline-grid;
		place-items: center;
		flex: none;
		width: var(--s);
		height: var(--s);
		border-radius: var(--r-sm);
		font-family: var(--font-mono);
		font-stretch: 87.5%;
		font-size: calc(var(--s) * 0.34);
		font-weight: 500;
		line-height: 1;
		color: var(--ink);
		background: var(--key);
		box-shadow: inset 0 0 0 1px var(--line-2);
		user-select: none;
	}
	.me::after {
		content: '';
		position: absolute;
		right: 2px;
		bottom: 2px;
		width: max(3px, calc(var(--s) * 0.14));
		height: max(3px, calc(var(--s) * 0.14));
		background: var(--led);
	}
	.crown {
		position: absolute;
		top: calc(var(--s) * -0.5);
		left: 50%;
		translate: -50% 0;
		display: grid;
		color: var(--accent);
		pointer-events: none;
	}
	/* Tiny avatars sit in text lines: the crown perches on the shoulder instead of on top. */
	.small .crown {
		top: -7px;
		left: auto;
		right: -7px;
		translate: none;
	}
</style>
