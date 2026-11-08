<script lang="ts">
	import { Tooltip } from 'bits-ui';
	import type { Station } from '#lib/station.svelte.ts';
	import { fmtCountdown, showCountdown, syncLevel } from '#lib/game.ts';
	import { motion, pop } from '#lib/motion/motion.ts';

	/** Game HUD chips: connection, sync readout (quirk 5), up-next countdown (quirk 8). */
	let { station }: { station: Station } = $props();

	const statusLabel = { connecting: 'Connecting', live: 'Live', reconnecting: 'Reconnecting' };
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
	const counting = $derived(showCountdown(remaining, !!s?.playing, !!next));
	const secs = $derived(counting ? fmtCountdown(remaining) : '');

	// Countdown: slide in, then a pulse on every second.
	let up = $state<HTMLElement>();
	let prevSecs = '';
	$effect(() => {
		if (secs && prevSecs && secs !== prevSecs) pop(up, 0.08);
		prevSecs = secs;
	});
	function chipIn(el: HTMLElement) {
		motion(el, { y: [-14, 0], scale: [0.8, 1], opacity: [0, 1], duration: 420, ease: 'outBack(2)' });
	}
</script>

<div class="hud">
	<span class="chip status {station.status}" role="status">
		<span class="pip" aria-hidden="true"></span>
		{statusLabel[station.status]}
	</span>

	<Tooltip.Root>
		<Tooltip.Trigger class="chip sync {level}" aria-label="Sync {drift === null ? 'not playing' : `${Math.round(drift)} ms`}">
			<span class="pip" aria-hidden="true"></span>
			Sync <b>{drift === null ? '—' : `${Math.round(drift)} ms`}</b>
		</Tooltip.Trigger>
		<Tooltip.Portal>
			<Tooltip.Content sideOffset={8} class="cb-tip">
				How far your speaker is from the shared timeline. Under 10 ms is perfect; past 25 ms it quietly resyncs.
			</Tooltip.Content>
		</Tooltip.Portal>
	</Tooltip.Root>

	{#if counting && next}
		<span class="chip upnext" bind:this={up} use:chipIn>
			Up next in <b>{secs}</b>
			<span class="t">{next.track.title}</span>
		</span>
	{/if}
</div>

<style>
	.hud {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}
	.hud :global(.chip) {
		flex: none;
	}
	.status.live .pip {
		background: var(--blush);
		box-shadow: 0 0 0 0 color-mix(in oklab, var(--blush) 60%, transparent);
		animation: live 1.8s ease-out infinite;
	}
	.status.live {
		color: var(--ink);
	}
	.status.reconnecting .pip {
		background: var(--warn);
		animation: blink 1s ease-in-out infinite;
	}
	@keyframes live {
		to {
			box-shadow: 0 0 0 7px transparent;
		}
	}
	@keyframes blink {
		50% {
			opacity: 0.25;
		}
	}
	.hud :global(.sync) {
		cursor: help;
	}
	.hud :global(.sync.good .pip) {
		background: var(--ok);
	}
	.hud :global(.sync.fair .pip) {
		background: var(--warn);
	}
	.hud :global(.sync.poor .pip) {
		background: var(--bad);
	}
	.hud :global(.sync b) {
		min-width: 3.4em;
	}
	.upnext {
		flex: 0 1 auto !important;
		min-width: 0;
		background: var(--accent-soft);
		border-color: color-mix(in oklab, var(--accent) 45%, transparent);
		color: var(--accent-ink);
	}
	.upnext .t {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		text-transform: none;
		letter-spacing: 0;
		font-weight: 550;
		font-size: 0.82rem;
		color: var(--ink);
		max-width: 22ch;
	}
	@media (max-width: 560px) {
		.hud {
			gap: 6px;
		}
		.upnext .t {
			display: none;
		}
	}
</style>
