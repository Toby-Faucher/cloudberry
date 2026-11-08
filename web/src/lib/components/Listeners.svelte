<script lang="ts">
	import { Tooltip } from 'bits-ui';
	import { scale } from 'svelte/transition';
	import { flip } from 'svelte/animate';
	import { backOut } from 'svelte/easing';
	import type { Station } from '#lib/station.svelte.ts';
	import { dur, hue, initials } from '#lib/util.ts';

	let { station }: { station: Station } = $props();
	const listeners = $derived(station.state?.listeners ?? []);
</script>

<div class="room" aria-label="{listeners.length} listening">
	<ul>
		{#each listeners as l (l.id)}
			<li
				animate:flip={{ duration: dur(260) }}
				in:scale={{ start: 0.4, duration: dur(380), easing: backOut }}
				out:scale={{ start: 0.6, duration: dur(180) }}
			>
				<Tooltip.Root>
					<Tooltip.Trigger
						class={['avatar', l.id === station.you && 'me']}
						style="--h: {hue(l.name)}"
						aria-label={l.id === station.you ? `${l.name} (you)` : l.name}
					>
						{initials(l.name)}
					</Tooltip.Trigger>
					<Tooltip.Portal>
						<Tooltip.Content sideOffset={8} class="cb-tip">
							{l.name}{l.id === station.you ? ' (you)' : ''}
						</Tooltip.Content>
					</Tooltip.Portal>
				</Tooltip.Root>
			</li>
		{/each}
	</ul>
	<span class="count num">{listeners.length} listening</span>
</div>

<style>
	.room {
		display: flex;
		align-items: center;
		gap: 10px;
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
	}
	li + li {
		margin-left: -8px;
	}
	ul :global(.avatar) {
		display: grid;
		place-items: center;
		width: 32px;
		height: 32px;
		border-radius: 50%;
		font-size: 0.72rem;
		font-weight: 600;
		letter-spacing: 0.02em;
		color: oklch(0.32 0.08 var(--h));
		background: oklch(0.88 0.07 var(--h));
		border: 2px solid var(--bg);
		cursor: default;
		transition: transform 160ms var(--ease-out);
	}
	ul :global(.avatar.me) {
		box-shadow: 0 0 0 2px var(--accent);
	}
	@media (hover: hover) {
		ul :global(.avatar:hover) {
			transform: translateY(-2px);
		}
	}
	@media (prefers-color-scheme: dark) {
		ul :global(.avatar) {
			color: oklch(0.9 0.06 var(--h));
			background: oklch(0.38 0.07 var(--h));
		}
	}
	.count {
		font-size: 0.82rem;
		color: var(--ink-2);
	}
	@media (max-width: 480px) {
		.count {
			display: none;
		}
	}
	:global(.cb-tip) {
		background: var(--ink);
		color: var(--bg);
		font-size: 0.8rem;
		padding: 5px 10px;
		border-radius: 8px;
		box-shadow: var(--shadow);
		z-index: 50;
		transform-origin: var(--bits-tooltip-content-transform-origin);
		animation: tip-in 140ms var(--ease-out);
	}
	@keyframes tip-in {
		from {
			opacity: 0;
			transform: scale(0.94);
		}
	}
</style>
