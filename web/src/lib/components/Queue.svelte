<script lang="ts">
	import { flip } from 'svelte/animate';
	import { fly, scale } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import { dndzone, SHADOW_ITEM_MARKER_PROPERTY_NAME, type DndEvent } from 'svelte-dnd-action';
	import { ChevronDown, ChevronUp, GripVertical, X } from '@lucide/svelte';
	import type { Station } from '#lib/station.svelte.ts';
	import type { QueueItem } from '#lib/types.ts';
	import { dur, fmtTime } from '#lib/util.ts';
	import Art from './Art.svelte';
	import Avatar from './Avatar.svelte';

	let { station }: { station: Station } = $props();

	type Row = { id: number; q: QueueItem; [SHADOW_ITEM_MARKER_PROPERTY_NAME]?: boolean };
	// Server order, overridden locally while dragging and until the next state arrives
	// (the server's answer always wins).
	let items = $derived<Row[]>((station.state?.queue ?? []).map((q) => ({ id: q.queue_id, q })));
	const flipMs = dur(220);

	function consider(e: CustomEvent<DndEvent<Row>>) {
		items = e.detail.items;
	}
	function finalize(e: CustomEvent<DndEvent<Row>>) {
		items = e.detail.items;
		const id = Number(e.detail.info.id);
		const to = items.findIndex((r) => r.id === id);
		const from = station.state?.queue.findIndex((q) => q.queue_id === id) ?? -1;
		if (to >= 0 && from >= 0 && to !== from) station.send({ type: 'move', queue_id: id, to_index: to });
	}

	/** Buttons (keyboard / touch fallback). to_index = final 0-based position in the upcoming queue. */
	function move(row: Row, from: number, to: number) {
		if (to < 0 || to >= items.length || from === to) return;
		const next = [...items];
		next.splice(to, 0, ...next.splice(from, 1));
		items = next;
		station.send({ type: 'move', queue_id: row.id, to_index: to });
	}
</script>

<section aria-labelledby="queue-h">
	<h2 class="section-title" id="queue-h">
		Up next <small class="num">{items.length ? `${items.length} ${items.length === 1 ? 'track' : 'tracks'}` : ''}</small>
	</h2>

	{#if items.length === 0}
		<p class="empty" in:fly={{ y: 4, duration: dur(200) }}>Nothing queued. Add a link above or pick something from the library.</p>
	{:else}
		<ol
			class="list"
			aria-label="Queue, drag to reorder"
			use:dndzone={{
				items,
				flipDurationMs: flipMs,
				type: 'queue',
				dropTargetStyle: {},
				transformDraggedElement: (el) => el?.classList.add('queue-dragged')
			}}
			onconsider={consider}
			onfinalize={finalize}
		>
			{#each items as row, i (row.id)}
				<li
					class="row"
					class:shadow={row[SHADOW_ITEM_MARKER_PROPERTY_NAME]}
					animate:flip={{ duration: flipMs, easing: cubicOut }}
					in:fly={{ x: -12, duration: dur(320), easing: cubicOut }}
					out:scale={{ start: 0.96, duration: dur(200), opacity: 0 }}
				>
					<span class="grip" aria-hidden="true"><GripVertical size={16} /></span>
					<Art track={row.q.track} size={40} />
					<span class="what">
						<span class="name" title={row.q.track.title}>{row.q.track.title}</span>
						<span class="sub">
							<Avatar name={row.q.added_by} size={16} />
							<span class="by">{row.q.added_by}</span> · <span class="num">{fmtTime(row.q.track.duration_ms)}</span>
						</span>
					</span>
					<span class="actions">
						<button class="icon-btn" aria-label="Move {row.q.track.title} up" disabled={i === 0} onclick={() => move(row, i, i - 1)}>
							<ChevronUp size={16} />
						</button>
						<button
							class="icon-btn"
							aria-label="Move {row.q.track.title} down"
							disabled={i === items.length - 1}
							onclick={() => move(row, i, i + 1)}
						>
							<ChevronDown size={16} />
						</button>
						<button class="icon-btn remove" aria-label="Remove {row.q.track.title} from queue" onclick={() => station.remove(row.q)}>
							<X size={16} />
						</button>
					</span>
				</li>
			{/each}
		</ol>
	{/if}
</section>

<style>
	.empty {
		margin: 0;
		padding: 18px;
		border-radius: var(--r-md);
		border: 1px dashed var(--line);
		color: var(--ink-3);
		font-size: 0.9rem;
		text-align: center;
	}
	.list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		gap: 4px;
	}
	.list:focus-visible {
		outline-offset: 4px;
	}
	.row {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 8px 8px 8px 4px;
		border-radius: var(--r-md);
		background: var(--surface);
		border: 1px solid transparent;
		transition:
			background-color 160ms var(--ease-out),
			border-color 160ms var(--ease-out);
	}
	@media (hover: hover) {
		.row:hover {
			border-color: var(--line);
		}
		.row:hover .grip {
			opacity: 1;
		}
	}
	/* The gap the dragged row will drop into. */
	.row.shadow {
		visibility: visible !important;
		background: transparent;
		border: 1px dashed color-mix(in oklab, var(--accent) 50%, var(--line));
	}
	.row.shadow > * {
		visibility: hidden;
	}
	/* The floating clone the library appends to <body>. */
	:global(.queue-dragged) {
		box-shadow:
			0 0 0 1px var(--line),
			0 18px 40px -16px rgb(60 30 10 / 0.35) !important;
		cursor: grabbing;
	}
	.grip {
		color: var(--ink-3);
		cursor: grab;
		opacity: 0.4;
		transition: opacity 160ms;
		display: grid;
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
	}
	.sub {
		display: flex;
		align-items: center;
		gap: 6px;
		color: var(--ink-3);
		font-size: 0.8rem;
		white-space: nowrap;
		min-width: 0;
	}
	.by {
		overflow: hidden;
		text-overflow: ellipsis;
		color: var(--ink-2);
	}
	.actions {
		display: flex;
		flex: none;
	}
	.remove:hover:not(:disabled) {
		color: var(--danger);
	}
	@media (pointer: coarse) {
		.grip {
			display: none;
		}
		.row {
			padding-left: 8px;
		}
	}
</style>
