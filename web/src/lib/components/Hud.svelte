<script lang="ts">
	import type { Station } from '#lib/station.svelte.ts';
	import { fmtCountdown, showCountdown, syncLevel } from '#lib/game.ts';
	import { motion } from '#lib/motion/motion.ts';

	/**
	 * Header readouts: a four-segment sync meter (quirk 5; ms on hover, focus or tap) and the
	 * up-next countdown (quirk 8). Connection state lives on the screen's status lamp.
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

	// Countdown: slides in, then the digits roll down a notch every second.
	let digits = $state<HTMLElement>();
	let prevSecs = '';
	$effect(() => {
		if (secs && prevSecs && secs !== prevSecs) motion(digits, { y: [-9, 0], opacity: [0.2, 1], duration: 160, ease: 'outQuart' });
		prevSecs = secs;
	});
	function slideIn(el: HTMLElement) {
		motion(el, { x: [-8, 0], opacity: [0, 1], duration: 220, ease: 'outQuart' });
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
		<span class="label">Sync</span>
		<span class="bars" aria-hidden="true">
			{#each [0, 1, 2, 3] as i (i)}<span class="b" class:lit={i < lit}></span>{/each}
		</span>
		<span class="ms num" aria-hidden="true">{ms || '—'}</span>
	</button>

	{#if counting && next}
		<span class="upnext" use:slideIn>
			<span class="label">Next in</span>
			<span class="win"><b class="num" bind:this={digits}>{secs}</b></span>
			<span class="t">{next.track.title}</span>
		</span>
	{/if}
</div>

<style>
	.hud {
		display: flex;
		align-items: center;
		gap: 20px;
		min-width: 0;
	}
	.meter {
		flex: none;
		display: inline-flex;
		align-items: center;
		gap: 8px;
		height: 32px;
		padding: 0 8px;
		margin-left: -8px;
		border-radius: var(--r-md);
	}
	@media (hover: hover) {
		.meter:hover {
			background: var(--surface-2);
		}
	}
	/* Four equal LED segments, like a level meter. */
	.bars {
		display: flex;
		gap: 2px;
	}
	.b {
		width: 4px;
		height: 12px;
		background: var(--led-off);
		transition: background-color 200ms var(--ease-out);
	}
	.good .b.lit {
		background: var(--ink);
	}
	.fair .b.lit {
		background: var(--accent);
	}
	.poor .b.lit {
		background: var(--danger);
	}
	/* The number only when you ask: hover, keyboard focus, or a tap. */
	.ms {
		display: none;
		font-family: var(--font-mono);
		font-size: 0.6875rem;
		color: var(--ink);
		min-width: 4.2em;
		text-align: left;
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
		gap: 10px;
		height: 32px;
		padding-left: 20px;
		border-left: 1px solid var(--line);
		white-space: nowrap;
	}
	/* The digits sit in a window so a roll clips at its edge. */
	.win {
		display: inline-grid;
		overflow: hidden;
		height: 1.5rem;
		align-items: center;
	}
	.win b {
		display: block;
		font-family: var(--font-dot);
		font-weight: 800;
		font-size: 1.375rem;
		line-height: 1;
		color: var(--accent-ink);
	}
	.upnext .t {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		font-size: 0.875rem;
		font-weight: 500;
		color: var(--ink);
		max-width: 26ch;
	}
	@media (max-width: 560px) {
		.upnext .t {
			display: none;
		}
	}
</style>
