<script lang="ts">
	// The mark: seven drupelets in a 2-3-2 hex cluster, drawn like a sticker: flat body, flat
	// shade, one white highlight per bead (hard stops, no soft gradients). Still: no idle motion.
	let { bead = 22, opacity = 1 }: { bead?: number | string; opacity?: number } = $props();
	const rows = [2, 3, 2];
</script>

<div class="berry" style:--b={typeof bead === "number" ? `${bead}px` : bead} style:opacity aria-hidden="true">
	{#each rows as n, r (r)}
		<div class="row">
			{#each Array(n) as _, i (i)}<span></span>{/each}
		</div>
	{/each}
</div>

<style>
	.berry {
		display: grid;
		justify-items: center;
	}
	.row {
		display: flex;
		gap: calc(var(--b) * 0.06);
	}
	.row + .row {
		margin-top: calc(var(--b) * -0.12);
	}
	span {
		width: var(--b);
		height: var(--b);
		border-radius: 50%;
		background:
			radial-gradient(circle at 34% 30%, #fffdf9 0 15%, transparent 16%),
			radial-gradient(circle at 40% 36%, var(--berry-400) 0 58%, var(--berry-600) 59%);
	}
</style>
