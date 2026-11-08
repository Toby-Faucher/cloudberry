<script lang="ts">
	import { flip } from 'svelte/animate';
	import { fly } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import type { Station } from '#lib/station.svelte.ts';
	import type { Track } from '#lib/types.ts';
	import { dur, fmtTime } from '#lib/util.ts';
	import Art from './Art.svelte';
	import Avatar from './Avatar.svelte';
	import Icon from '#lib/icons/Icon.svelte';
	import PlaylistPicker from './PlaylistPicker.svelte';

	/** Everything anyone ever added, newest first: a ruled table. */
	let { station }: { station: Station } = $props();
	const LIMIT = 8;
	const library = $derived(station.library);
	let expanded = $state(false);
	const shown = $derived(expanded ? library : library.slice(0, LIMIT));
	// Brief "Queued" confirmation per track.
	let flashed = $state<string | null>(null);

	function enqueue(t: Track) {
		station.send({ type: 'enqueue', track_id: t.id });
		flashed = t.id;
		setTimeout(() => flashed === t.id && (flashed = null), 1400);
	}
</script>

<section aria-labelledby="lib-h">
	<h2 class="section-title" id="lib-h">Library <span class="count num">{String(library.length).padStart(2, '0')}</span></h2>
	{#if library.length === 0}
		<p class="empty">Nothing here yet. Every song anyone adds ends up here.</p>
	{:else}
		<div class="table">
			<ul>
				{#each shown as t (t.id)}
					<li animate:flip={{ duration: dur(280), easing: cubicOut }} in:fly={{ y: 8, duration: dur(320), easing: cubicOut }}>
						<button class="row" onclick={() => enqueue(t)} aria-label="Add {t.title} to the queue">
							<Art track={t} size={32} />
							<span class="name" title={t.title}>{t.title}</span>
							<span class="by"><Avatar name={t.added_by} size={18} /><span>{t.added_by}</span></span>
							<span class="dur num">{fmtTime(t.duration_ms)}</span>
							<span class="add" class:done={flashed === t.id} aria-hidden="true">
								<Icon name={flashed === t.id ? 'check' : 'plus'} size={14} />
							</span>
						</button>
						<PlaylistPicker {station} ids={[t.id]} label="Add {t.title} to a playlist" triggerClass="icon-btn pl" />
					</li>
				{/each}
			</ul>
			{#if library.length > LIMIT}
				<button class="more" onclick={() => (expanded = !expanded)} aria-expanded={expanded}>
					{expanded ? 'Fewer' : `All ${library.length} songs`}
				</button>
			{/if}
		</div>
	{/if}
</section>

<style>
	.empty {
		margin: 0;
		padding: 12px 0;
		color: var(--ink-2);
		font-size: 0.9375rem;
	}
	ul {
		list-style: none;
		margin: -12px 0 0;
		padding: 0;
	}
	li {
		display: flex;
		align-items: center;
		border-bottom: 1px solid var(--line);
	}
	li :global(.pl) {
		flex: none;
		width: 32px;
		height: 32px;
	}
	.row {
		flex: 1;
		min-width: 0;
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) minmax(0, 8rem) 3.2rem auto;
		align-items: center;
		gap: 14px;
		padding: 8px 0;
		text-align: left;
		transition: background-color 120ms var(--ease-out);
	}
	@media (hover: hover) {
		.row:hover {
			background: color-mix(in oklab, var(--bg) 60%, var(--surface));
		}
		.row:hover .add {
			background: var(--ink);
			color: var(--surface);
			box-shadow: none;
		}
	}
	.name {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		font-weight: 500;
		font-size: 0.9375rem;
	}
	.by {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
		font-family: var(--font-mono);
		font-stretch: 87.5%;
		font-size: 0.6875rem;
		color: var(--ink-2);
		white-space: nowrap;
	}
	.by span {
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.dur {
		text-align: right;
		font-family: var(--font-mono);
		font-size: 0.75rem;
		color: var(--ink-2);
	}
	.add {
		display: grid;
		place-items: center;
		width: 30px;
		height: 30px;
		margin-right: 4px;
		border-radius: var(--r-md);
		color: var(--ink);
		box-shadow: inset 0 0 0 1px var(--line-2);
		transition:
			background-color 120ms var(--ease-out),
			color 120ms var(--ease-out);
	}
	.add.done {
		background: var(--accent);
		color: var(--on-accent);
		box-shadow: none;
	}
	.more {
		display: flex;
		align-items: center;
		width: 100%;
		height: 44px;
		font-family: var(--font-mono);
		font-stretch: 87.5%;
		font-size: 0.6875rem;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--ink);
		text-decoration: underline;
		text-decoration-color: var(--line-2);
		text-underline-offset: 4px;
	}
	@media (hover: hover) {
		.more:hover {
			text-decoration-color: var(--ink);
		}
	}
	@media (max-width: 560px) {
		.row {
			grid-template-columns: auto minmax(0, 1fr) auto auto auto;
			gap: 10px;
		}
		.by span {
			display: none; /* the tile says who */
		}
	}
</style>
