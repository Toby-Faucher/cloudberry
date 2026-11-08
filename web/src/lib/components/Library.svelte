<script lang="ts">
	import { flip } from 'svelte/animate';
	import { fly } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import type { Station } from '#lib/station.svelte.ts';
	import type { Track } from '#lib/types.ts';
	import { dur, fmtTime } from '#lib/util.ts';
	import Art from './Art.svelte';
	import Icon from './Icon.svelte';

	let { station }: { station: Station } = $props();
	const library = $derived(station.state?.library ?? []);
	// Brief "Queued" confirmation per track.
	let flashed = $state<string | null>(null);

	function enqueue(t: Track) {
		station.send({ type: 'enqueue', track_id: t.id });
		flashed = t.id;
		setTimeout(() => flashed === t.id && (flashed = null), 1400);
	}
</script>

<section aria-labelledby="lib-h">
	<h2 class="section-title" id="lib-h">
		Library <small class="num">{library.length || ''}</small>
	</h2>
	{#if library.length === 0}
		<p class="empty">Every track anyone adds lands here, ready to queue again.</p>
	{:else}
		<ul class="grid">
			{#each library as t (t.id)}
				<li animate:flip={{ duration: dur(280), easing: cubicOut }} in:fly={{ y: 8, duration: dur(320), easing: cubicOut }}>
					<button class="tile" onclick={() => enqueue(t)} aria-label="Add {t.title} to the queue">
						<Art track={t} size={44} />
						<span class="what">
							<span class="name" title={t.title}>{t.title}</span>
							<span class="sub"><span class="num">{fmtTime(t.duration_ms)}</span> · {t.added_by}</span>
						</span>
						<span class="add" class:done={flashed === t.id} aria-hidden="true">
							<Icon name={flashed === t.id ? 'check' : 'plus'} size={16} />
						</span>
					</button>
				</li>
			{/each}
		</ul>
	{/if}
</section>

<style>
	.empty {
		margin: 0;
		color: var(--ink-3);
		font-size: 0.9rem;
	}
	.grid {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
		gap: 6px;
	}
	.tile {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 8px;
		border-radius: var(--r-md);
		text-align: left;
		transition:
			background-color 160ms var(--ease-out),
			transform 120ms var(--ease-out);
	}
	@media (hover: hover) {
		.tile:hover {
			background: var(--surface);
		}
		.tile:hover .add {
			opacity: 1;
		}
	}
	.tile:active {
		transform: scale(0.98);
	}
	.what {
		flex: 1;
		min-width: 0;
		display: grid;
	}
	.name {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		font-weight: 500;
		font-size: 0.92rem;
	}
	.sub {
		color: var(--ink-3);
		font-size: 0.78rem;
	}
	.add {
		display: grid;
		place-items: center;
		width: 28px;
		height: 28px;
		flex: none;
		border-radius: 50%;
		color: var(--accent-ink);
		background: var(--accent-soft);
		opacity: 0;
		transition:
			opacity 160ms var(--ease-out),
			background-color 200ms;
	}
	.add.done {
		opacity: 1;
		background: var(--accent);
		color: var(--on-accent);
	}
	@media (hover: none) {
		.add {
			opacity: 1;
		}
	}
</style>
