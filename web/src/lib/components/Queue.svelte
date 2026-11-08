<script lang="ts">
	import { flip } from 'svelte/animate';
	import { fly } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import { dndzone, SHADOW_ITEM_MARKER_PROPERTY_NAME, type DndEvent } from 'svelte-dnd-action';
	import Icon from '#lib/icons/Icon.svelte';
	import type { Station } from '#lib/station.svelte.ts';
	import type { QueueItem } from '#lib/types.ts';
	import { dur, fmtTime } from '#lib/util.ts';
	import Art from './Art.svelte';
	import { onMount } from 'svelte';
	import { smear } from '#lib/motion/smear.ts';
	import { smearAnimate } from '#lib/motion/motion.ts';
	import PlaylistPicker from './PlaylistPicker.svelte';

	let { station }: { station: Station } = $props();

	type Row = { id: number; q: QueueItem; [SHADOW_ITEM_MARKER_PROPERTY_NAME]?: boolean };
	// Server order, overridden locally while dragging and until the next state arrives
	// (the server's answer always wins).
	let items = $derived<Row[]>((station.state?.queue ?? []).map((q) => ({ id: q.queue_id, q })));
	const flipMs = dur(220);

	let list = $state<HTMLElement>();
	// New rows slide in (smeared) once the list is on screen; the first paint just appears.
	let ready = false;
	onMount(() => (ready = true));
	function rowIn(el: HTMLElement) {
		if (ready) smearAnimate(el, { x: [-80, 0], opacity: [0, 1], duration: 320, ease: 'outQuart' });
	}
	function rowOut(node: Element) {
		return {
			duration: dur(180),
			css: (t: number) => `opacity:${t};translate:${(1 - t) * 40}px 0`
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
		<span class="title-right">
			{#if items.length}
				<PlaylistPicker
					{station}
					ids={items.map((r) => r.q.track.id)}
					label="Save the queue as a playlist"
					newOnly
					defaultName={`Up next, ${new Date().toLocaleDateString('en', { month: 'short', day: 'numeric' })}`}
					triggerClass="title-key"
				>
					<Icon name="list-plus" size={14} /> Save
				</PlaylistPicker>
			{/if}
			<span class="count num">{String(items.length).padStart(2, '0')}</span>
		</span>
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
					<span class="idx num" aria-hidden="true"><span class="grip"><Icon name="grip" size={16} /></span><span class="n">{String(i + 1).padStart(2, '0')}</span></span>
					<Art track={row.q.track} size={40} />
					<span class="what">
						<span class="name" title={row.q.track.title}>{row.q.track.title}</span>
						<span class="sub">
							{#if i === 0}<span class="deck-tag"><span class="led on"></span>Next</span>{/if}
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
							<Icon name="chev-up" size={18} />
						</button>
						<button
							class="icon-btn"
							class:gone={i === items.length - 1}
							aria-label="Move {row.q.track.title} down"
							title="Move down"
							disabled={i === items.length - 1}
							onclick={() => move(row, i, i + 1)}
						>
							<Icon name="chev-down" size={18} />
						</button>
						<PlaylistPicker {station} ids={[row.q.track.id]} label="Add {row.q.track.title} to a playlist" />
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
		padding: 24px 0;
		color: var(--ink-2);
		font-size: 0.9375rem;
	}
	/* Up next: numbered rows on hairlines, like a sequencer's step list. */
	.list {
		list-style: none;
		margin: -12px 0 0;
		padding: 0;
	}
	.list:focus-visible {
		outline-offset: 4px;
	}
	.row {
		border-bottom: 1px solid var(--line);
	}
	.body {
		display: flex;
		align-items: center;
		gap: 14px;
		min-height: 60px;
		padding: 10px 4px 10px 0;
		background: var(--bg);
		transition: background-color 120ms var(--ease-out);
	}
	.idx {
		position: relative;
		flex: none;
		width: 2.4em;
		font-family: var(--font-mono);
		font-size: 0.75rem;
		color: var(--ink-3);
	}
	.next .idx {
		color: var(--accent-ink);
	}
	.grip {
		position: absolute;
		left: -2px;
		top: 50%;
		translate: 0 -50%;
		display: grid;
		color: var(--ink-2);
		cursor: grab;
		opacity: 0;
		transition: opacity 120ms;
	}
	@media (hover: hover) and (pointer: fine) {
		.row:hover .body {
			background: color-mix(in oklab, var(--bg) 60%, var(--surface));
		}
		.row:hover .grip {
			opacity: 1;
		}
		.row:hover .n {
			opacity: 0;
		}
	}
	.dur {
		flex: none;
		min-width: 3.4em;
		text-align: right;
		font-family: var(--font-mono);
		font-size: 0.75rem;
		color: var(--ink-2);
	}
	/* The gap the dragged row will drop into. */
	.row.shadow {
		visibility: visible !important;
	}
	.row.shadow .body {
		background: transparent;
		outline: 1px dashed var(--line-2);
		outline-offset: -1px;
	}
	.row.shadow .body > * {
		visibility: hidden;
	}
	/* The floating clone the library appends to <body>: lifted off the panel (a real shadow). */
	:global(.queue-dragged) {
		background: var(--surface) !important;
		box-shadow: var(--shadow-lift) !important;
		cursor: grabbing;
	}
	:global(.queue-dragged .body) {
		background: var(--surface) !important;
		padding-left: 8px !important;
	}
	.what {
		flex: 1;
		min-width: 0;
		display: grid;
		gap: 3px;
	}
	.name {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		font-weight: 550;
	}
	.sub {
		display: flex;
		align-items: center;
		gap: 10px;
		font-family: var(--font-mono);
		font-stretch: 87.5%;
		font-size: 0.6875rem;
		color: var(--ink-2);
		white-space: nowrap;
		min-width: 0;
	}
	.deck-tag {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		color: var(--accent-ink);
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}
	.by {
		overflow: hidden;
		text-overflow: ellipsis;
	}
	/* Row actions: out of the way until you point at or tab into the row. */
	.actions {
		display: flex;
		flex: none;
		gap: 2px;
		opacity: 0;
		transition: opacity 120ms var(--ease-out);
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
		}
	}
	@media (pointer: coarse) {
		.actions {
			opacity: 1;
		}
		.grip {
			display: none;
		}
	}
	/* Phones: the time moves under the title so the row buttons don't squeeze the name away. */
	.dur-sm {
		display: none;
		margin-left: auto;
		padding-left: 8px;
	}
	@media (max-width: 560px) {
		.body {
			gap: 10px;
		}
		.idx {
			width: 1.8em;
		}
		.dur {
			display: none;
		}
		.dur-sm {
			display: inline;
		}
	}
</style>
