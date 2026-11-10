<script lang="ts">
	import { ui } from '#lib/ui.svelte.ts';
	// The real sticker mark (web/static/logo-mark.png, 96×96, tinted to your accent via ui.logo). `bead` is the old drupelet size;
	// the mark spans ~3 beads so existing call sites keep their footprint. Decorative: the
	// wordmark or the surrounding UI names it.
	let { bead = 22, opacity = 1 }: { bead?: number | string; opacity?: number } = $props();
</script>

<img
	class="berry"
	src={ui.logo}
	alt=""
	width="96"
	height="96"
	draggable="false"
	style:--b={typeof bead === 'number' ? `${bead}px` : bead}
	style:opacity
/>

<style>
	.berry {
		display: block;
		width: calc(var(--b) * 3);
		height: calc(var(--b) * 3);
		object-fit: contain;
		user-select: none;
	}
</style>
