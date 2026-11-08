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

	/** Everything anyone ever added, newest first: one plain list on a cream sheet. */
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
	<h2 class="section-title" id="lib-h">Library</h2>
	{#if library.length === 0}
		<p class="empty">Nothing here yet. Every song anyone adds ends up here.</p>
	{:else}
		<div class="sheet">
			<ul>
				{#each shown as t (t.id)}
					<li animate:flip={{ duration: dur(280), easing: cubicOut }} in:fly={{ y: 8, duration: dur(320), easing: cubicOut }}>
						<button class="row" onclick={() => enqueue(t)} aria-label="Add {t.title} to the queue">
							<Art track={t} size={40} />
							<span class="name" title={t.title}>{t.title}</span>
							<span class="by"><Avatar name={t.added_by} size={20} /><span>{t.added_by}</span></span>
							<span class="dur num">{fmtTime(t.duration_ms)}</span>
							<span class="add" class:done={flashed === t.id} aria-hidden="true">
								<Icon name={flashed === t.id ? 'check' : 'plus'} size={16} />
							</span>
						</button>
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
		color: var(--on-bg-2);
		font-size: 0.95rem;
	}
	.sheet {
		background: var(--surface);
		color: var(--ink);
		border-radius: 20px;
		padding: 6px;
		box-shadow: var(--sticker);
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	li + li {
		border-top: 2px solid var(--surface-2);
	}
	.row {
		width: 100%;
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) minmax(0, 9rem) 3rem auto;
		align-items: center;
		gap: 14px;
		padding: 8px 8px;
		border-radius: 12px;
		text-align: left;
		transition: background-color 160ms var(--ease-out);
	}
	@media (hover: hover) {
		.row:hover {
			background: var(--surface-2);
		}
		.row:hover .add {
			background: var(--accent);
			color: var(--on-accent);
		}
	}
	.name {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		font-weight: 600;
		font-size: 0.95rem;
	}
	.by {
		display: flex;
		align-items: center;
		gap: 7px;
		min-width: 0;
		color: var(--ink-2);
		font-size: 0.85rem;
		white-space: nowrap;
	}
	.by span {
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.dur {
		text-align: right;
		font-family: var(--font-display);
		font-weight: 600;
		color: var(--ink-2);
	}
	.add {
		display: grid;
		place-items: center;
		width: 34px;
		height: 34px;
		border-radius: 10px;
		color: var(--accent-ink);
		background: var(--accent-soft);
		transition:
			background-color 160ms var(--ease-out),
			color 160ms var(--ease-out);
	}
	.add.done {
		background: var(--accent);
		color: var(--on-accent);
	}
	.more {
		display: block;
		width: 100%;
		margin-top: 4px;
		padding: 10px;
		border-radius: 12px;
		border-top: 2px solid var(--surface-2);
		font-family: var(--font-display);
		font-weight: 650;
		color: var(--accent-ink);
	}
	@media (hover: hover) {
		.more:hover {
			background: var(--surface-2);
		}
	}
	@media (max-width: 560px) {
		.row {
			grid-template-columns: auto minmax(0, 1fr) auto auto auto;
			gap: 10px;
		}
		.by span {
			display: none; /* the avatar says who */
		}
	}
</style>
