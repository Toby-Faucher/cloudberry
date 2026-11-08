<script lang="ts">
	import { flip } from 'svelte/animate';
	import { fly } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import { dndzone, SHADOW_ITEM_MARKER_PROPERTY_NAME, type DndEvent } from 'svelte-dnd-action';
	import { ChevronDown, ChevronUp, GripVertical, X } from '@lucide/svelte';
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

	function consider(e: CustomEvent<DndEvent<Row>>) {
		items = e.detail.items;
	}
	function finalize(e: CustomEvent<DndEvent<Row>>) {
		items = e.detail.items;
		const id = Number(e.detail.info.id);
		const to = items.findIndex((r) => r.id === id);
		const from = station.state?.queue.findIndex((q) => q.queue_id === id) ?? -1;
		if (to >= 0 && from >= 0 && to !== from) station.send({ type: 'move', queue_id: id, to_index: to });
		smearReorder();
	}

	/** Buttons (keyboard / touch fallback). to_index = final 0-based position in the upcoming queue. */
	function move(row: Row, from: number, to: number) {
		if (to < 0 || to >= items.length || from === to) return;
		const next = [...items];
		next.splice(to, 0, ...next.splice(from, 1));
		smearReorder();
		items = next;
		station.send({ type: 'move', queue_id: row.id, to_index: to });
	}
</script>

<section aria-labelledby="queue-h">
	<h2 class="section-title" id="queue-h">
		Up next <small class="num" aria-label="{items.length} {items.length === 1 ? 'track' : 'tracks'}">{items.length || ''}</small>
	</h2>

	{#if items.length === 0}
		<p class="empty" in:fly={{ y: 4, duration: dur(200) }}>Nothing queued. Add a link above or pick something from the library.</p>
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
							{#if i === 0}<span class="tag">Next</span>{/if}{row.q.track.title}
						</span>
						<span class="sub">
							<Avatar name={row.q.added_by} size={16} />
							<span class="by">{row.q.added_by}</span> · <span class="num">{fmtTime(row.q.track.duration_ms)}</span>
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
							<X size={18} />
						</button>
					</span>
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
		border: 1px dashed var(--line);
		color: var(--ink-2);
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
		border-radius: var(--r-md);
	}
	.body {
		display: flex;
		align-items: center;
		gap: 12px;
		min-height: 60px;
		padding: 8px 8px 8px 4px;
		border-radius: var(--r-md);
		border: 2px solid transparent;
		transition:
			background-color 160ms var(--ease-out),
			border-color 160ms var(--ease-out);
	}
	/* The next track reads as "on deck": berry-tinted and labelled. */
	.next .body {
		background: color-mix(in oklab, var(--accent-soft) 70%, var(--surface));
		border-color: color-mix(in oklab, var(--accent) 35%, transparent);
	}
	.tag {
		display: inline-block;
		margin-right: 8px;
		padding: 2px 7px;
		border-radius: 6px;
		background: var(--accent);
		color: var(--on-accent);
		font-size: 0.66rem;
		font-weight: 750;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		vertical-align: 2px;
	}
	@media (hover: hover) {
		.row:hover .body {
			background: var(--surface);
		}
		.row.next:hover .body {
			border-color: color-mix(in oklab, var(--accent) 60%, transparent);
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
		border: 2px dashed color-mix(in oklab, var(--accent) 60%, var(--line));
	}
	.row.shadow .body > * {
		visibility: hidden;
	}
	/* The floating clone the library appends to <body>. */
	:global(.queue-dragged) {
		box-shadow: var(--shadow-pop) !important;
		cursor: grabbing;
		rotate: -1.5deg;
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
</style>
