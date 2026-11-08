<script lang="ts">
	import type { Station } from '#lib/station.svelte.ts';
	import { fmtCountdown, showCountdown, syncLevel } from '#lib/game.ts';
	import { motion, pop } from '#lib/motion/motion.ts';

	/**
	 * Header HUD: a tiny signal meter for sync (quirk 5; ms on hover, focus or tap) and the
	 * up-next countdown sticker (quirk 8). Connection state lives on the art as the on-air sticker.
	 */
	let { station }: { station: Station } = $props();

	const s = $derived(station.state);
	const next = $derived(s?.queue[0] ?? null);

	// Sampled, never tweened: a counting-up number would lie about drift.
	let drift = $state<number | null>(null);
	let remaining = $state(Infinity);
	$effect(() => {
		const id = setInterval(() => {
			const st = station.state;
			const playing = !!st?.playing && !!st.current;
			drift = playing ? Math.abs(station.player.lastDrift) : null;
			remaining = playing ? st!.current!.track.duration_ms - station.positionMs() : Infinity;
		}, 250);
		return () => clearInterval(id);
	});
	const level = $derived(syncLevel(drift));
	const lit = $derived({ good: 4, fair: 3, poor: 1, off: 0 }[level]);
	const words = { good: 'tight', fair: 'a bit loose', poor: 'drifting, resyncing', off: 'not playing' };
	const ms = $derived(drift === null ? '' : `${Math.round(drift)} ms`);
	let peek = $state(false);

	const counting = $derived(showCountdown(remaining, !!s?.playing, !!next));
	const secs = $derived(counting ? fmtCountdown(remaining) : '');

	// Countdown: slap on, then a pulse on every second.
	let up = $state<HTMLElement>();
	let prevSecs = '';
	$effect(() => {
		if (secs && prevSecs && secs !== prevSecs) pop(up, 0.08);
		prevSecs = secs;
	});
	function slapIn(el: HTMLElement) {
		motion(el, { y: [-14, 0], scale: [0.8, 1], rotate: [-8, -2], opacity: [0, 1], duration: 420, ease: 'outBack(2)', clear: false });
	}
</script>

<div class="hud">
	<button
		class="meter {level}"
		class:peek
		aria-label="Sync {ms || 'idle'}, {words[level]}"
		title="How far your speaker is from everyone else's. Under 10 ms is spot on."
		onclick={() => (peek = !peek)}
	>
		<span class="bars" aria-hidden="true">
			{#each [0, 1, 2, 3] as i (i)}<span class="b" class:lit={i < lit} style:--h="{6 + i * 4}px"></span>{/each}
		</span>
		<span class="ms num" aria-hidden="true">{ms || '—'}</span>
	</button>

	{#if counting && next}
		<span class="upnext" bind:this={up} use:slapIn>
			<span class="in">next in <b class="num">{secs}</b></span>
			<span class="t">{next.track.title}</span>
		</span>
	{/if}
</div>

<style>
	.hud {
		display: flex;
		align-items: center;
		gap: 12px;
		min-width: 0;
	}
	/* Signal meter: four chunky bars on a small cream tile. */
	.meter {
		flex: none;
		display: inline-flex;
		align-items: flex-end;
		gap: 8px;
		height: 36px;
		padding: 0 10px 8px;
		border-radius: 10px;
		background: var(--surface);
		box-shadow: 0 4px 0 var(--shade);
		color: var(--ink-3);
	}
	.bars {
		display: flex;
		align-items: flex-end;
		gap: 3px;
	}
	.b {
		width: 5px;
		height: var(--h);
		border-radius: 2px;
		background: color-mix(in oklab, var(--ink-3) 30%, transparent);
		transition: background-color 300ms var(--ease-out);
	}
	.good .b.lit {
		background: var(--ok);
	}
	.fair .b.lit {
		background: var(--accent-line);
	}
	.poor .b.lit {
		background: var(--danger);
	}
	/* The number only when you ask: hover, keyboard focus, or a tap. */
	.ms {
		display: none;
		font-family: var(--font-display);
		font-weight: 650;
		font-size: 0.9rem;
		line-height: 1;
		color: var(--ink);
		min-width: 3.4em;
	}
	.meter.peek .ms,
	.meter:focus-visible .ms {
		display: block;
	}
	@media (hover: hover) {
		.meter:hover .ms {
			display: block;
		}
	}

	.upnext {
		flex: 0 1 auto;
		min-width: 0;
		display: inline-flex;
		align-items: center;
		gap: 8px;
		height: 36px;
		padding: 0 12px;
		border-radius: 10px;
		background: var(--accent-soft);
		color: var(--ink);
		box-shadow:
			0 0 0 3px var(--die),
			0 5px 0 3px var(--shade);
		font-family: var(--font-display);
		font-weight: 600;
		white-space: nowrap;
	}
	.in b {
		color: var(--accent-ink);
		font-weight: 700;
	}
	.upnext .t {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		font-family: var(--font-body);
		font-size: 0.85rem;
		font-weight: 550;
		max-width: 22ch;
	}
	@media (max-width: 560px) {
		.upnext .t {
			display: none;
		}
	}
</style>
