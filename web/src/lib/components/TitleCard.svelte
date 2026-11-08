<script lang="ts">
	import type { Station } from '#lib/station.svelte.ts';
	import { reduced, smearAnimate } from '#lib/motion/motion.ts';
	import { viz } from '#lib/visualizer/state.svelte.ts';

	/**
	 * Quirk 2: on a track change the title drops in big and centred on a graphite plate (smeared),
	 * holds ~1.2 s, then flies into its spot in the deck (#np-title). Never blocks input.
	 */
	let { station }: { station: Station } = $props();

	const qid = $derived(station.state?.current?.queue_id ?? null);
	let card = $state<{ title: string; by: string; seq: number } | null>(null);
	let el = $state<HTMLElement>();
	let last: number | null | undefined; // undefined until the first state (joining isn't a change)
	let seq = 0;

	const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

	$effect(() => {
		const q = qid;
		if (!station.state) return;
		const changed = last !== undefined && q !== last;
		last = q;
		const cur = station.state.current;
		if (!changed || !cur || reduced || document.hidden || viz.lean) return;
		void play({ title: cur.track.title, by: cur.added_by, seq: ++seq });
	});

	async function play(c: { title: string; by: string; seq: number }) {
		card = c;
		await Promise.resolve(); // let the card mount
		await new Promise(requestAnimationFrame);
		if (!el || seq !== c.seq) return;
		await smearAnimate(el, { y: [-240, 0], opacity: [0, 1], duration: 340, ease: 'outQuart', clear: false });
		await wait(1200);
		if (seq !== c.seq || !el) return;
		const target = document.getElementById('np-title');
		const from = el.getBoundingClientRect();
		const to = target?.getBoundingClientRect();
		const h1 = el.querySelector('h2');
		const ratio =
			target && h1 ? parseFloat(getComputedStyle(target).fontSize) / parseFloat(getComputedStyle(h1).fontSize) : 0.4;
		const dx = to ? to.left + to.width / 2 - (from.left + from.width / 2) : 0;
		const dy = to ? to.top + to.height / 2 - (from.top + from.height / 2) : 200;
		await smearAnimate(el, {
			x: [0, dx],
			y: [0, dy],
			scale: [1, Math.max(0.2, ratio)],
			opacity: [1, 1, 0.9, 0],
			duration: 420,
			ease: 'inOutQuart',
			clear: false
		});
		if (seq === c.seq) card = null;
	}
</script>

{#if card}
	{#key card.seq}
		<div class="layer" aria-hidden="true">
			<div class="slab" bind:this={el}>
				<p class="top"><span class="led on"></span>On air</p>
				<h2>{card.title}</h2>
				<p class="by">Added by <strong>{card.by}</strong></p>
			</div>
		</div>
	{/key}
{/if}

<style>
	.layer {
		position: fixed;
		inset: 0;
		z-index: 60;
		display: grid;
		place-items: center;
		pointer-events: none;
		padding: 24px;
	}
	.slab {
		max-width: min(92vw, 880px);
		padding: 20px 28px 26px;
		border-radius: var(--r-lg);
		background: var(--ink);
		color: var(--surface);
		box-shadow: var(--shadow-lift);
		opacity: 0;
	}
	.top,
	.by {
		margin: 0;
		font-family: var(--font-mono);
		font-stretch: 87.5%;
		font-size: 0.75rem;
		letter-spacing: 0.06em;
		text-transform: uppercase;
	}
	.top {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-bottom: 14px;
	}
	h2 {
		font-size: clamp(2.2rem, 1.4rem + 3.6vw, 4.5rem);
		font-weight: 600;
		line-height: 1;
		letter-spacing: -0.035em;
		text-wrap: balance;
		display: -webkit-box;
		-webkit-line-clamp: 3;
		line-clamp: 3;
		-webkit-box-orient: vertical;
		overflow: hidden;
		padding-bottom: 0.06em;
	}
	.by {
		margin-top: 16px;
	}
	.by strong {
		font-weight: 600;
		color: var(--berry-400);
	}
</style>
