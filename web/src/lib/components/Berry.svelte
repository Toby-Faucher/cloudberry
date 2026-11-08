<script lang="ts">
	// The mark: seven beads in a 2-3-2 hex cluster, like the fruit. They breathe out of phase.
	let { bead = 22, opacity = 0.85 }: { bead?: number; opacity?: number } = $props();
	const rows = [2, 3, 2];
</script>

<div class="berry" style:--b="{bead}px" style:--o={opacity} aria-hidden="true">
	{#each rows as n, r}
		<div class="row">
			{#each Array(n) as _, i}<span style:--i={r * 3 + i}></span>{/each}
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
		gap: calc(var(--b) * 0.16);
	}
	.row + .row {
		margin-top: calc(var(--b) * -0.06);
	}
	span {
		width: var(--b);
		height: var(--b);
		border-radius: 50%;
		background: var(--accent);
		opacity: var(--o);
		animation: breathe 2.8s calc(var(--i) * 0.18s) var(--ease-in-out) infinite;
	}
	@keyframes breathe {
		50% {
			transform: scale(0.84);
			opacity: calc(var(--o) * 0.6);
		}
	}
</style>
