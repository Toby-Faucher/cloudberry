<script lang="ts">
	import { Tooltip } from 'bits-ui';
	import { scale } from 'svelte/transition';
	import { flip } from 'svelte/animate';
	import { quartOut } from 'svelte/easing';
	import type { Station } from '#lib/station.svelte.ts';
	import { dur } from '#lib/util.ts';
	import { ui } from '#lib/ui.svelte.ts';
	import { bounce, wiggle } from '#lib/motion/motion.ts';
	import Avatar from './Avatar.svelte';

	/** Who's here: avatar stack (click someone to poke them), away fades, DJ crown, +N overflow. */
	let { station, max = 5 }: { station: Station; max?: number } = $props();
	const listeners = $derived(station.state?.listeners ?? []);
	// You first, then whoever is actually here before whoever is away.
	const sorted = $derived(
		[...listeners].sort((a, b) => Number(b.id === station.you) - Number(a.id === station.you) || Number(a.away) - Number(b.away))
	);
	const shown = $derived(sorted.slice(0, max));
	const extra = $derived(sorted.slice(max));
	let root = $state<HTMLElement>();

	const el = (id: number) => root?.querySelector<HTMLElement>(`[data-lid="${id}"]`);
	// Pokes wiggle the target on everyone's screen; reacting makes your avatar hop.
	$effect(() => {
		if (station.poked) wiggle(el(station.poked.id));
	});
	$effect(() => {
		if (station.reacted) bounce(el(station.reacted.id));
	});

	const label = (l: { id: number; name: string; away: boolean }) =>
		[l.name + (l.id === station.you ? ' (you)' : ''), l.away && 'away', ui.dj === l.name && 'DJ of the day']
			.filter(Boolean)
			.join(', ');
</script>

<div class="room" bind:this={root}>
	<ul aria-label="{listeners.length} listening">
		{#each shown as l (l.id)}
			<li
				animate:flip={{ duration: dur(260) }}
				in:scale={{ start: 0.7, duration: dur(180), easing: quartOut }}
				out:scale={{ start: 0.7, duration: dur(120) }}
			>
				<Tooltip.Root>
					<Tooltip.Trigger
						class={['who', l.away && 'away', l.id !== station.you && 'pokable']}
						data-lid={l.id}
						aria-label={l.id === station.you ? label(l) : `Poke ${label(l)}`}
						onclick={() => station.poke(l.id)}
					>
						<Avatar name={l.name} me={l.id === station.you} size={28} crown />
					</Tooltip.Trigger>
					<Tooltip.Portal>
						<Tooltip.Content sideOffset={8} class="cb-tip">
							{label(l)}{l.id === station.you ? '' : '. Click to poke'}
						</Tooltip.Content>
					</Tooltip.Portal>
				</Tooltip.Root>
			</li>
		{/each}
		{#if extra.length}
			<li>
				<Tooltip.Root>
					<Tooltip.Trigger class="more num" aria-label="{extra.length} more: {extra.map((l) => l.name).join(', ')}">
						+{extra.length}
					</Tooltip.Trigger>
					<Tooltip.Portal>
						<Tooltip.Content sideOffset={8} class="cb-tip">{extra.map(label).join(', ')}</Tooltip.Content>
					</Tooltip.Portal>
				</Tooltip.Root>
			</li>
		{/if}
	</ul>
</div>

<style>
	.room {
		display: flex;
		align-items: center;
	}
	/* A row of printed tiles, not a stack of bubbles. */
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		gap: 4px;
	}
	ul :global(.who),
	ul :global(.more) {
		position: relative;
		display: grid;
		place-items: center;
		border-radius: var(--r-sm);
		cursor: default;
		transition: opacity 240ms var(--ease-out);
	}
	ul :global(.who.pokable) {
		cursor: pointer;
	}
	ul :global(.who.away) {
		opacity: 0.4;
	}
	@media (hover: hover) {
		ul :global(.who.pokable:hover .avatar) {
			box-shadow: inset 0 0 0 1px var(--ink);
		}
	}
	ul :global(.more) {
		width: 28px;
		height: 28px;
		background: var(--surface-2);
		color: var(--ink);
		font-family: var(--font-mono);
		font-size: 0.625rem;
	}
</style>
