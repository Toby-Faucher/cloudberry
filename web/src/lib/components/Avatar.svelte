<script lang="ts">
	import { Crown } from '@lucide/svelte';
	import { hue, initials } from '#lib/util.ts';
	import { ui } from '#lib/ui.svelte.ts';

	/** Decorative by default: the name is almost always printed next to it. DJ of the day wears the crown. */
	let { name, size = 32, me = false }: { name: string; size?: number; me?: boolean } = $props();
	const dj = $derived(ui.dj !== null && ui.dj === name);
</script>

<span class="avatar" class:me class:small={size < 24} style:--h={hue(name)} style:--s="{size}px" aria-hidden="true"
	>{initials(name)}{#if dj}<span class="crown"><Crown size={size < 24 ? 10 : Math.round(size * 0.5)} fill="currentColor" strokeWidth={2.2} /></span>{/if}</span
>

<style>
	.avatar {
		position: relative;
		display: inline-grid;
		place-items: center;
		flex: none;
		width: var(--s);
		height: var(--s);
		border-radius: 50%;
		font-size: calc(var(--s) * 0.36);
		font-weight: 650;
		letter-spacing: 0.02em;
		line-height: 1;
		/* Fixed L/C, hue per name: dark-on-light passes AA in both themes. */
		color: oklch(0.34 0.09 var(--h));
		background: oklch(0.88 0.075 var(--h));
		user-select: none;
	}
	.me {
		box-shadow: 0 0 0 2px var(--accent-line);
	}
	.crown {
		position: absolute;
		top: calc(var(--s) * -0.42);
		left: 50%;
		translate: -50% 0;
		rotate: -12deg;
		display: grid;
		color: var(--berry-500);
		filter: drop-shadow(0 1px 0 var(--berry-700)) drop-shadow(0 1px 2px rgb(0 0 0 / 0.3));
		pointer-events: none;
	}
	/* Tiny avatars sit in text lines: the crown perches on the shoulder instead of on top. */
	.small .crown {
		top: -6px;
		left: auto;
		right: -7px;
		translate: none;
		rotate: 20deg;
	}
	@media (prefers-color-scheme: dark) {
		.avatar {
			color: oklch(0.92 0.06 var(--h));
			background: oklch(0.42 0.08 var(--h));
		}
	}
</style>
