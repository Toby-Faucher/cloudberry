<script lang="ts">
	import { flip } from 'svelte/animate';
	import { fly, scale } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import type { Station } from '#lib/station.svelte.ts';
	import type { QueueItem } from '#lib/types.ts';
	import { dur, fmtTime } from '#lib/util.ts';
	import Art from './Art.svelte';
	import Icon from './Icon.svelte';

	let { station }: { station: Station } = $props();

	// Server order, overridden locally while dragging / until the next state arrives (optimistic).
	let items = $derived<QueueItem[]>(station.state?.queue ?? []);
	let dragId = $state<number | null>(null);
	let dragFrom = -1;

	function moveLocal(from: number, to: number) {
		const next = [...items];
		next.splice(to, 0, ...next.splice(from, 1));
		items = next;
	}

	/** to_index = final 0-based position in the upcoming queue. */
	function move(item: QueueItem, from: number, to: number) {
		if (to < 0 || to >= items.length || from === to) return;
		moveLocal(from, to);
		station.send({ type: 'move', queue_id: item.queue_id, to_index: to });
	}

	function onDragStart(e: DragEvent, item: QueueItem, i: number) {
		dragId = item.queue_id;
		dragFrom = i;
		e.dataTransfer!.effectAllowed = 'move';
		e.dataTransfer!.setData('text/plain', String(item.queue_id));
	}
	function onDragOver(e: DragEvent, i: number) {
		if (dragId === null) return;
		e.preventDefault();
		const from = items.findIndex((q) => q.queue_id === dragId);
		if (from !== i) moveLocal(from, i); // live reorder; flip animates neighbours
	}
	function onDragEnd() {
		if (dragId === null) return;
		const to = items.findIndex((q) => q.queue_id === dragId);
		if (to !== dragFrom && to >= 0) station.send({ type: 'move', queue_id: dragId, to_index: to });
		dragId = null;
	}
</script>

<section aria-labelledby="queue-h">
	<h2 class="section-title" id="queue-h">
		Up next <small class="num">{items.length ? `${items.length} ${items.length === 1 ? 'track' : 'tracks'}` : ''}</small>
	</h2>

	{#if items.length === 0}
		<p class="empty" in:fly={{ y: 4, duration: dur(200) }}>Nothing queued. Add a link above or pick something from the library.</p>
	{:else}
		<ol class="list">
			{#each items as item, i (item.queue_id)}
				<li
					class="row"
					class:dragging={dragId === item.queue_id}
					draggable="true"
					ondragstart={(e) => onDragStart(e, item, i)}
					ondragover={(e) => onDragOver(e, i)}
					ondrop={(e) => e.preventDefault()}
					ondragend={onDragEnd}
					animate:flip={{ duration: dur(280), easing: cubicOut }}
					in:fly={{ x: -12, duration: dur(320), easing: cubicOut }}
					out:scale={{ start: 0.96, duration: dur(200), opacity: 0 }}
				>
					<span class="grip" aria-hidden="true"><Icon name="grip" size={16} /></span>
					<Art track={item.track} size={40} />
					<span class="what">
						<span class="name" title={item.track.title}>{item.track.title}</span>
						<span class="sub"><span class="num">{fmtTime(item.track.duration_ms)}</span> · {item.added_by}</span>
					</span>
					<span class="actions">
						<button class="icon-btn" aria-label="Move {item.track.title} up" disabled={i === 0} onclick={() => move(item, i, i - 1)}>
							<Icon name="up" size={16} />
						</button>
						<button
							class="icon-btn"
							aria-label="Move {item.track.title} down"
							disabled={i === items.length - 1}
							onclick={() => move(item, i, i + 1)}
						>
							<Icon name="down" size={16} />
						</button>
						<button
							class="icon-btn remove"
							aria-label="Remove {item.track.title} from queue"
							onclick={() => station.send({ type: 'remove', queue_id: item.queue_id })}
						>
							<Icon name="x" size={16} />
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
			box-shadow 200ms var(--ease-out),
			opacity 160ms;
	}
	@media (hover: hover) {
		.row:hover {
			border-color: var(--line);
		}
		.row:hover .grip {
			opacity: 1;
		}
	}
	.row.dragging {
		opacity: 0.55;
		box-shadow: 0 0 0 2px var(--accent);
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
		color: var(--ink-3);
		font-size: 0.8rem;
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
