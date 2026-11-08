<script lang="ts">
	import type { Station } from '#lib/station.svelte.ts';
	import { reduced, smearAnimate } from '#lib/motion/motion.ts';
	import { viz } from '#lib/visualizer/state.svelte.ts';

	/**
	 * Quirk 2: on a track change the title slams in big and centred (smeared), holds ~1.2 s, then
	 * flies into its spot in the now-playing card (#np-title). Never blocks input.
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
		await smearAnimate(el, { y: [-320, 0], scale: [1.5, 1], rotate: [-8, -2], opacity: [0, 1], duration: 560, ease: 'outBack(1.5)', clear: false });
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
			rotate: [-2, 0],
			opacity: [1, 1, 0.9, 0],
			duration: 480,
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
				<h2>{card.title}</h2>
				<p>added by <strong>{card.by}</strong></p>
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
		padding: 26px 34px 24px;
		border-radius: 28px;
		background: var(--accent);
		color: var(--on-accent);
		text-align: center;
		box-shadow: var(--lip), var(--sticker);
		opacity: 0;
	}
	h2 {
		font-size: clamp(2.2rem, 1.4rem + 3.6vw, 4.4rem);
		font-weight: 700;
		line-height: 1.02;
		letter-spacing: -0.015em;
		text-wrap: balance;
		display: -webkit-box;
		-webkit-line-clamp: 3;
		line-clamp: 3;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	p {
		margin: 12px 0 0;
		font-size: clamp(0.95rem, 0.85rem + 0.4vw, 1.15rem);
		font-weight: 550;
	}
</style>
