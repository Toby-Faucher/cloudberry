<script lang="ts">
	/** A m:ss time on the dot-matrix face. Doto's own colon is a cross of dots; this draws two single dots. */
	let { text }: { text: string } = $props();
	const parts = $derived(text.split(':'));
</script>

<span class="dot-time num">{#each parts as p, i (i)}{#if i}<span class="colon" aria-hidden="true"></span>{/if}{p}{/each}</span>

<style>
	.dot-time {
		font-family: var(--font-dot);
		font-weight: 800;
		white-space: nowrap;
	}
	.colon {
		display: inline-block;
		width: 0.42em;
		height: 0.7em;
		/* two square dots, sized like Doto's own at this weight */
		background:
			linear-gradient(currentColor, currentColor) center 22% / 0.1em 0.1em no-repeat,
			linear-gradient(currentColor, currentColor) center 78% / 0.1em 0.1em no-repeat;
	}
</style>
