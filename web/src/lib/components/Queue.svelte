<script lang="ts">
	import { flip } from 'svelte/animate';
	import { fly } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import { dndzone, SHADOW_ITEM_MARKER_PROPERTY_NAME, type DndEvent } from 'svelte-dnd-action';
	import { ChevronDown, ChevronUp, GripVertical } from '@lucide/svelte';
	import Icon from '#lib/icons/Icon.svelte';
	import type { Station } from '#lib/station.svelte.ts';
	import type { QueueItem } from '#lib/types.ts';
	import { dur, fmtTime } from '#lib/util.ts';
	import Art from './Art.svelte';
	import Avatar from './Avatar.svelte';
	import { onMount } from 'svelte';
	import { smear } from '#lib/motion/smear.ts';
	import { smearAnimate } from '#lib/motion/motion.ts';

	let { station }: { station: Station } = $props();

	type Row = { id: number; q: QueueItem; [SHADOW_ITEM_MARKER_PROPERTY_NAME]?: boolean };
	// Server order, overridden locally while dragging and until the next state arrives
	// (the server's answer always wins).
	let items = $derived<Row[]>((station.state?.queue ?? []).map((q) => ({ id: q.queue_id, q })));
	const flipMs = dur(220);

	let list = $state<HTMLElement>();
	// New rows fling in (smeared) once the list is on screen; the first paint just appears.
	let ready = false;
	onMount(() => (ready = true));
	function rowIn(el: HTMLElement) {
		if (ready) smearAnimate(el, { x: [-120, 0], scale: [0.9, 1], opacity: [0, 1], duration: 560 });
	}
	function rowOut(node: Element) {
		return {
			duration: dur(260),
			css: (t: number) => `opacity:${t};translate:${(1 - t) * 60}px 0;scale:${0.9 + 0.1 * t}`
		};
	}
	/** Smear rows while the reorder FLIP plays (they move fast, briefly). */
	function smearReorder() {
		const rows = [...(list?.querySelectorAll<HTMLElement>('.body') ?? [])];
		rows.forEach((r) => smear.track(r));
		setTimeout(() => rows.forEach((r) => smear.untrack(r)), flipMs + 120);
	}

	// The server silently drops moves past its 10 actions / 5 s limit, and then no state comes
	// back to undo our local order. So shortly after the last move, snap back to server order.
	let dragging = false;
	let resync: ReturnType<typeof setTimeout> | undefined;
	function sendMove(queue_id: number, to_index: number) {
		station.send({ type: 'move', queue_id, to_index });
		clearTimeout(resync);
		resync = setTimeout(() => {
			if (!dragging) items = (station.state?.queue ?? []).map((q) => ({ id: q.queue_id, q }));
		}, 1500);
	}

	function consider(e: CustomEvent<DndEvent<Row>>) {
		dragging = true;
		items = e.detail.items;
	}
	function finalize(e: CustomEvent<DndEvent<Row>>) {
		dragging = false;
		items = e.detail.items;
		const id = Number(e.detail.info.id);
		const to = items.findIndex((r) => r.id === id);
		const from = station.state?.queue.findIndex((q) => q.queue_id === id) ?? -1;
		if (to >= 0 && from >= 0 && to !== from) sendMove(id, to);
		smearReorder();
	}

	/** Buttons (keyboard / touch fallback). to_index = final 0-based position in the upcoming queue. */
	function move(row: Row, from: number, to: number) {
		if (to < 0 || to >= items.length || from === to) return;
		const next = [...items];
		next.splice(to, 0, ...next.splice(from, 1));
		smearReorder();
		items = next;
		sendMove(row.id, to);
	}
</script>

<section aria-labelledby="queue-h">
	<h2 class="section-title" id="queue-h">
		Up next
	</h2>

	{#if items.length === 0}
		<p class="empty" in:fly={{ y: 4, duration: dur(200) }}>Nobody's queued anything. The aux is up for grabs.</p>
	{:else}
		<ol
			bind:this={list}
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
					class:next={i === 0}
					class:shadow={row[SHADOW_ITEM_MARKER_PROPERTY_NAME]}
					animate:flip={{ duration: flipMs, easing: cubicOut }}
					out:rowOut
				>
					<div class="body" use:rowIn>
					<span class="grip" aria-hidden="true"><GripVertical size={16} /></span>
					<Art track={row.q.track} size={40} />
					<span class="what">
						<span class="name" title={row.q.track.title}>
							{#if i === 0}<span class="tag">next</span>{/if}{row.q.track.title}
						</span>
						<span class="sub">
							<Avatar name={row.q.added_by} size={18} />
							<span class="by">{row.q.added_by}</span>
							<span class="dur-sm num">{fmtTime(row.q.track.duration_ms)}</span>
						</span>
					</span>
					<span class="actions">
						<button
							class="icon-btn"
							class:gone={i === 0}
							aria-label="Move {row.q.track.title} up"
							title="Move up"
							disabled={i === 0}
							onclick={() => move(row, i, i - 1)}
						>
							<ChevronUp size={18} />
						</button>
						<button
							class="icon-btn"
							class:gone={i === items.length - 1}
							aria-label="Move {row.q.track.title} down"
							title="Move down"
							disabled={i === items.length - 1}
							onclick={() => move(row, i, i + 1)}
						>
							<ChevronDown size={18} />
						</button>
						<button
							class="icon-btn remove"
							aria-label="Remove {row.q.track.title} from queue"
							title="Remove (with undo)"
							onclick={() => station.remove(row.q)}
						>
							<Icon name="close" size={16} />
						</button>
					</span>
					<span class="dur num">{fmtTime(row.q.track.duration_ms)}</span>
					</div>
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
		border: 3px dashed color-mix(in oklab, var(--on-bg) 45%, transparent);
		color: var(--on-bg);
		font-family: var(--font-display);
		font-size: 1.05rem;
		font-weight: 550;
		text-align: center;
	}
	/* Up next: a stack of cream tiles on the bog, each on its own hard drop. */
	.list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		gap: 10px;
	}
	.list:focus-visible {
		outline-offset: 4px;
	}
	.row {
		border-radius: var(--r-md);
	}
	.body {
		display: flex;
		align-items: center;
		gap: 12px;
		min-height: 62px;
		padding: 8px 8px 8px 4px;
		border-radius: var(--r-md);
		background: var(--surface);
		color: var(--ink);
		box-shadow: 0 4px 0 var(--shade);
		transition: background-color 160ms var(--ease-out);
	}
	/* The next track is on deck: berry-soft, die-cut, labelled. */
	.next .body {
		background: var(--accent-soft);
		box-shadow:
			0 0 0 3px var(--die),
			0 6px 0 3px var(--shade);
	}
	.tag {
		display: inline-block;
		margin-right: 8px;
		padding: 0 8px 1px;
		border-radius: 6px;
		background: var(--accent);
		color: var(--on-accent);
		font-family: var(--font-display);
		font-size: 0.85rem;
		font-weight: 700;
		rotate: -4deg;
		vertical-align: 1px;
	}
	.dur {
		flex: none;
		min-width: 3.2em;
		padding-right: 8px;
		text-align: right;
		font-family: var(--font-display);
		font-weight: 600;
		color: var(--ink-2);
	}
	@media (hover: hover) {
		.row:not(.next):hover .body {
			background: color-mix(in oklab, var(--surface) 70%, var(--surface-2));
		}
		.row:hover .grip {
			opacity: 1;
		}
	}
	/* The gap the dragged row will drop into. */
	.row.shadow {
		visibility: visible !important;
	}
	.row.shadow .body {
		background: transparent;
		box-shadow: none;
		outline: 3px dashed color-mix(in oklab, var(--on-bg) 60%, transparent);
		outline-offset: -3px;
	}
	.row.shadow .body > * {
		visibility: hidden;
	}
	/* The floating clone the library appends to <body>: peeled off, held up. */
	:global(.queue-dragged) {
		box-shadow:
			0 0 0 3px var(--die),
			0 12px 0 3px var(--shade) !important;
		cursor: grabbing;
		rotate: -2.5deg;
	}
	.grip {
		color: var(--ink-3);
		cursor: grab;
		opacity: 0.5;
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
		font-weight: 600;
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
	/* Row actions: out of the way until you point at or tab into the row. */
	.actions {
		display: flex;
		flex: none;
		gap: 4px;
		opacity: 0;
		transition: opacity 140ms var(--ease-out);
	}
	.row:hover .actions,
	.row:focus-within .actions {
		opacity: 1;
	}
	.actions .gone {
		visibility: hidden;
	}
	@media (hover: hover) {
		.remove:hover:not(:disabled) {
			color: var(--danger);
			background: color-mix(in oklab, var(--danger) 12%, transparent);
		}
	}
	@media (pointer: coarse) {
		.actions {
			opacity: 1;
		}
		.grip {
			display: none;
		}
		.body {
			padding-left: 8px;
		}
	}
	/* Phones: the time moves under the title so the row buttons don't squeeze the name away. */
	.dur-sm {
		display: none;
		margin-left: auto;
		padding-left: 8px;
		font-family: var(--font-display);
		font-weight: 600;
		color: var(--ink-2);
	}
	@media (max-width: 560px) {
		.dur {
			display: none;
		}
		.dur-sm {
			display: inline;
		}
	}
</style>
