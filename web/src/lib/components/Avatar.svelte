<script lang="ts">
	import { hue, initials } from '#lib/util.ts';

	/** Decorative by default: the name is almost always printed next to it. */
	let { name, size = 32, me = false }: { name: string; size?: number; me?: boolean } = $props();
</script>

<span class="avatar" class:me style:--h={hue(name)} style:--s="{size}px" aria-hidden="true">{initials(name)}</span>

<style>
	.avatar {
		display: inline-grid;
		place-items: center;
		flex: none;
		width: var(--s);
		height: var(--s);
		border-radius: 50%;
		font-size: calc(var(--s) * 0.36);
		font-weight: 600;
		letter-spacing: 0.02em;
		line-height: 1;
		/* Fixed L/C, hue per name: dark-on-light passes AA in both themes. */
		color: oklch(0.34 0.09 var(--h));
		background: oklch(0.88 0.075 var(--h));
		user-select: none;
	}
	.me {
		box-shadow: 0 0 0 2px var(--accent);
	}
	@media (prefers-color-scheme: dark) {
		.avatar {
			color: oklch(0.92 0.06 var(--h));
			background: oklch(0.42 0.08 var(--h));
		}
	}
</style>
