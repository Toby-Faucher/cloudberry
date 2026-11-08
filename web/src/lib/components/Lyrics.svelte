<script lang="ts">
	import { untrack } from 'svelte';
	import { slide } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import type { Station } from '#lib/station.svelte.ts';
	import { fetchLyrics, lineAt } from '#lib/lyrics.ts';
	import { ui } from '#lib/ui.svelte.ts';
	import { dur } from '#lib/util.ts';

	/** Synced lyrics window for the current track; renders nothing when there are none. */
	let { station }: { station: Station } = $props();

	// A primitive, so the effect below only reruns when the track actually changes.
	const id = $derived(station.state?.current?.track.id ?? null);

	// Fetch per track (cached by id). Stale answers for a previous track are dropped.
	$effect(() => {
		const tid = id;
		ui.lines = null;
		if (!tid) return;
		const title = untrack(() => station.state?.current?.track.title ?? '');
		let live = true;
		fetchLyrics(tid, title).then((lines) => live && (ui.lines = lines));
		return () => (live = false);
	});

	let idx = $state(-1);
	$effect(() => {
		const lines = ui.lines;
		if (!lines || !ui.lyrics) return;
		let raf = 0;
		const tick = () => {
			idx = lineAt(lines, station.positionMs());
			raf = requestAnimationFrame(tick);
		};
		tick();
		return () => cancelAnimationFrame(raf);
	});

	// Keep the current line about a third of the way down the window.
	let box = $state<HTMLElement>();
	let y = $state(0);
	$effect(() => {
		void ui.lines; // re-measure for a new track even if idx is unchanged
		const el = box?.querySelectorAll<HTMLElement>('li')[Math.max(0, idx)];
		if (el && box) y = box.clientHeight * 0.36 - el.offsetTop - el.offsetHeight / 2;
	});
</script>

{#if ui.lyrics && ui.lines}
	<div class="lyrics" bind:this={box} transition:slide={{ duration: dur(280), easing: cubicOut }} aria-label="Lyrics">
		<ol style:transform="translateY({y}px)">
			{#each ui.lines as line, i (i)}
				<li class:now={i === idx} class:past={i < idx} aria-current={i === idx ? 'true' : undefined}>
					{line.text || '♪'}
				</li>
			{/each}
		</ol>
	</div>
{/if}

<style>
	.lyrics {
		position: relative;
		height: 7.4rem;
		margin: 14px -8px 0;
		overflow: hidden;
		text-align: center;
		mask-image: linear-gradient(transparent, #000 26%, #000 64%, transparent);
	}
	ol {
		list-style: none;
		margin: 0;
		padding: 0 8px;
		transition: transform 520ms var(--ease-out);
	}
	li {
		padding: 3px 0;
		font-size: 0.95rem;
		line-height: 1.35;
		color: var(--ink-3);
		text-wrap: balance;
		transition:
			color 300ms var(--ease-out),
			opacity 300ms var(--ease-out);
	}
	li.past {
		opacity: 0.6;
	}
	li.now {
		color: var(--ink);
		font-weight: 600;
	}
	@media (prefers-reduced-motion: reduce) {
		ol {
			transition: none;
		}
	}
</style>
