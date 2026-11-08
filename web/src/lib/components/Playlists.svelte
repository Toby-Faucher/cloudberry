<script lang="ts">
	import { flip } from 'svelte/animate';
	import { fly } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import { tick } from 'svelte';
	import { dndzone, SHADOW_ITEM_MARKER_PROPERTY_NAME, type DndEvent } from 'svelte-dnd-action';
	import type { Station } from '#lib/station.svelte.ts';
	import type { Playlist, Track } from '#lib/types.ts';
	import { dur, fmtTime, isTyping } from '#lib/util.ts';
	import { enter } from '#lib/motion/motion.ts';
	import { smear } from '#lib/motion/smear.ts';
	import { dropMove, fmtTotal, moved, NAME_MAX, nameTaken, totalMs, validName } from '#lib/saved.ts';
	import Art from './Art.svelte';
	import Avatar from './Avatar.svelte';
	import Icon from '#lib/icons/Icon.svelte';
	import PlaylistPicker from './PlaylistPicker.svelte';

	/** Saved shared playlists: a numbered list; open one to edit and queue it. */
	let { station }: { station: Station } = $props();

	let openId = $state<number | null>(null);
	// Deleted (by anyone) while open: falls back to the list.
	const open = $derived(station.playlists.find((p) => p.id === openId) ?? null);
	const count = (p: Playlist) => p.track_ids.length;
	const songs = (n: number) => `${n} song${n === 1 ? '' : 's'}`;

	let root = $state<HTMLElement>();
	/** The list slides back in only when returning from a playlist, not on first paint. */
	let cameBack = $state(false);
	async function show(p: Playlist | null) {
		const back = openId;
		cameBack = !p;
		renaming = false;
		openId = p?.id ?? null;
		await tick();
		if (p) root?.querySelector<HTMLElement>('.back')?.focus();
		else root?.querySelector<HTMLElement>(`[data-pl="${back}"]`)?.focus();
	}

	// ---- detail: rows keyed by server index; local order wins while dragging / until the server answers
	type Row = { id: number; tid: string; t: Track | undefined; [SHADOW_ITEM_MARKER_PROPERTY_NAME]?: boolean };
	let items = $derived<Row[]>((open?.track_ids ?? []).map((tid, id) => ({ id, tid, t: station.byId.get(tid) })));
	const flipMs = dur(220);
	let list = $state<HTMLElement>();
	let dragging = false;
	let resync: ReturnType<typeof setTimeout> | undefined;
	const serverRows = () => (open?.track_ids ?? []).map((tid, id) => ({ id, tid, t: station.byId.get(tid) }));
	/** The server may drop an action (rate limit) and then nothing comes back: snap to its order. */
	function settle() {
		clearTimeout(resync);
		resync = setTimeout(() => !dragging && (items = serverRows()), 1500);
	}
	function smearRows() {
		const rows = [...(list?.querySelectorAll<HTMLElement>('.body') ?? [])];
		rows.forEach((r) => smear.track(r));
		setTimeout(() => rows.forEach((r) => smear.untrack(r)), flipMs + 120);
	}
	function consider(e: CustomEvent<DndEvent<Row>>) {
		dragging = true;
		items = e.detail.items;
	}
	function finalize(e: CustomEvent<DndEvent<Row>>) {
		dragging = false;
		items = e.detail.items;
		const m = open && dropMove(serverRows().map((r) => r.id), items.map((r) => r.id), Number(e.detail.info.id));
		if (m && open) {
			station.send({ type: 'playlist_move', id: open.id, ...m });
			settle();
		}
		smearRows();
	}
	function move(from: number, to: number) {
		if (!open || to < 0 || to >= items.length) return;
		smearRows();
		items = moved(items, from, to);
		station.send({ type: 'playlist_move', id: open.id, from, to });
		settle();
	}
	function remove(i: number) {
		if (!open) return;
		items = items.toSpliced(i, 1);
		station.send({ type: 'playlist_remove', id: open.id, index: i });
		settle();
	}

	// ---- rename in place
	let renaming = $state(false);
	let draft = $state('');
	function startRename() {
		draft = open?.name ?? '';
		renaming = true;
	}
	function commitRename() {
		if (!renaming) return;
		renaming = false;
		if (open && validName(draft) && draft.trim() !== open.name)
			station.send({ type: 'playlist_rename', id: open.id, name: draft.trim() });
	}
	function renameKey(e: KeyboardEvent) {
		if (e.key === 'Enter') commitRename();
		else if (e.key === 'Escape') {
			e.stopPropagation();
			renaming = false;
		}
	}

	function del(p: Playlist) {
		station.deletePlaylist(p);
		show(null);
	}

	function sectionKey(e: KeyboardEvent) {
		if (e.key === 'Escape' && open && !isTyping(e.target)) show(null);
	}
	const focusNow = (el: HTMLElement) => el.focus();
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<section aria-labelledby="pl-h" bind:this={root} onkeydown={sectionKey}>
	<h2 class="section-title" id="pl-h">
		Playlists
		<span class="title-right">
			{#if !open}
				<PlaylistPicker {station} ids={[]} label="New playlist" newOnly triggerClass="title-key">
					<Icon name="plus" size={14} /> New
				</PlaylistPicker>
			{/if}
			<span class="count num">{String(station.playlists.length).padStart(2, '0')}</span>
		</span>
	</h2>

	{#if open}
		{#key open.id}
			<div class="detail" use:enter={{ x: [24, 0], opacity: [0, 1], duration: 220, ease: 'outQuart' }}>
				<div class="head">
					<button class="icon-btn back" onclick={() => show(null)} aria-label="Back to all playlists" title="Back (Esc)">
						<Icon name="back" size={20} />
					</button>
					<div class="title-block">
						{#if renaming}
							<input
								class="rename"
								bind:value={draft}
								maxlength={NAME_MAX}
								aria-label="Playlist name"
								onkeydown={renameKey}
								onblur={commitRename}
								use:focusNow
							/>
							{#if validName(draft) && nameTaken(station.playlists, draft, open.id)}
								<span class="warn">Another playlist has that name. Allowed, just confusing.</span>
							{/if}
						{:else}
							<h3 class="pl-name">
								<button class="name-btn" onclick={startRename} aria-label="Rename {open.name}" title="Rename">
									<span>{open.name}</span><Icon name="pencil" size={14} />
								</button>
							</h3>
						{/if}
						<p class="meta">
							<span class="num">{songs(count(open))}</span>
							<span class="num">{fmtTotal(totalMs(open.track_ids, station.byId))}</span>
							<span class="by"><Avatar name={open.created_by} size={16} />{open.created_by}</span>
							{#if open.source_list_id}<span class="src"><span class="led on"></span>YouTube</span>{/if}
						</p>
					</div>
				</div>

				<div class="keys">
					<button class="go" disabled={!count(open)} onclick={() => open && station.queuePlaylist(open, false)}>
						<Icon name="play" size={16} /> Queue all
					</button>
					<button class="key shuffle" disabled={count(open) < 2} onclick={() => open && station.queuePlaylist(open, true)}>
						<Icon name="shuffle" size={18} /> Shuffle
					</button>
					<button class="icon-btn trash" onclick={() => open && del(open)} aria-label="Delete {open.name}" title="Delete (with undo)">
						<Icon name="trash" size={18} />
					</button>
				</div>

				{#if items.length === 0}
					<p class="empty">Nothing in here yet. Hit <Icon name="list-plus" size={14} /> on any song to drop it in.</p>
				{:else}
					<ol
						bind:this={list}
						class="tracks"
						aria-label="Songs in {open.name}, drag to reorder"
						use:dndzone={{
							items,
							flipDurationMs: flipMs,
							type: 'playlist',
							dropTargetStyle: {},
							transformDraggedElement: (el) => el?.classList.add('queue-dragged')
						}}
						onconsider={consider}
						onfinalize={finalize}
					>
						{#each items as row, i (row.id)}
							<li class="row" class:shadow={row[SHADOW_ITEM_MARKER_PROPERTY_NAME]} animate:flip={{ duration: flipMs, easing: cubicOut }}>
								<div class="body">
									<span class="idx num" aria-hidden="true"
										><span class="grip"><Icon name="grip" size={16} /></span><span class="n">{String(i + 1).padStart(2, '0')}</span></span
									>
									{#if row.t}<Art track={row.t} size={32} />{/if}
									<span class="name" title={row.t?.title ?? row.tid}>{row.t?.title ?? row.tid}</span>
									<span class="actions">
										<button class="icon-btn" class:gone={i === 0} disabled={i === 0} aria-label="Move {row.t?.title} up" title="Move up" onclick={() => move(i, i - 1)}>
											<Icon name="chev-up" size={18} />
										</button>
										<button
											class="icon-btn"
											class:gone={i === items.length - 1}
											disabled={i === items.length - 1}
											aria-label="Move {row.t?.title} down"
											title="Move down"
											onclick={() => move(i, i + 1)}
										>
											<Icon name="chev-down" size={18} />
										</button>
										<button class="icon-btn remove" aria-label="Remove {row.t?.title} from {open.name}" title="Remove" onclick={() => remove(i)}>
											<Icon name="close" size={16} />
										</button>
									</span>
									<span class="dur num">{fmtTime(row.t?.duration_ms ?? 0)}</span>
								</div>
							</li>
						{/each}
					</ol>
				{/if}
			</div>
		{/key}
	{:else if station.playlists.length === 0}
		<p class="empty" in:fly={{ y: 4, duration: dur(200) }}>No playlists yet. Start one for Friday afternoons, or paste a YouTube playlist and it saves itself.</p>
	{:else}
		<ul class="lists" use:enter={{ x: [-16, 0], opacity: [0, 1], duration: 200, ease: 'outQuart', skip: !cameBack }}>
			{#each station.playlists as p, i (p.id)}
				<li animate:flip={{ duration: dur(260), easing: cubicOut }} in:fly={{ y: 8, duration: dur(280), easing: cubicOut }}>
					<button class="pl-row" data-pl={p.id} onclick={() => show(p)} aria-label="Open {p.name}, {songs(count(p))}">
						<span class="idx num">{String(i + 1).padStart(2, '0')}</span>
						<span class="what">
							<span class="nm">{p.name}</span>
							<span class="sub">
								{#if p.source_list_id}<span class="src"><span class="led on"></span>YouTube</span>{/if}
								<span class="by-name">{p.created_by}</span>
							</span>
						</span>
						<span class="n num">{songs(count(p))}</span>
						<span class="dur num">{fmtTotal(totalMs(p.track_ids, station.byId))}</span>
						<span class="open" aria-hidden="true"><Icon name="back" size={16} /></span>
					</button>
				</li>
			{/each}
		</ul>
	{/if}
</section>

<style>
	.empty {
		margin: 0;
		padding: 12px 0;
		color: var(--ink-2);
		font-size: 0.9375rem;
	}
	.empty :global(.cb-icon) {
		display: inline;
		vertical-align: -2px;
	}

	/* ---- list ---- */
	.lists {
		list-style: none;
		margin: -12px 0 0;
		padding: 0;
	}
	.lists li {
		border-bottom: 1px solid var(--line);
	}
	.pl-row {
		width: 100%;
		display: grid;
		grid-template-columns: 2.4em minmax(0, 1fr) auto 4.4rem auto;
		align-items: center;
		gap: 14px;
		min-height: 56px;
		padding: 8px 0;
		text-align: left;
		transition: background-color 120ms var(--ease-out);
	}
	@media (hover: hover) {
		.pl-row:hover {
			background: color-mix(in oklab, var(--bg) 60%, var(--surface));
		}
		.pl-row:hover .open {
			color: var(--ink);
			translate: 2px 0;
		}
	}
	.idx {
		font-family: var(--font-mono);
		font-size: 0.75rem;
		color: var(--ink-3);
	}
	.what {
		display: grid;
		gap: 3px;
		min-width: 0;
	}
	.nm {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-weight: 550;
	}
	.sub,
	.meta {
		display: flex;
		align-items: center;
		gap: 10px;
		min-width: 0;
		font-family: var(--font-mono);
		font-stretch: 87.5%;
		font-size: 0.6875rem;
		color: var(--ink-2);
		white-space: nowrap;
	}
	.by-name {
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.src {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}
	.n,
	.dur {
		font-family: var(--font-mono);
		font-size: 0.75rem;
		color: var(--ink-2);
		text-align: right;
	}
	.open {
		display: grid;
		margin-right: 4px;
		color: var(--ink-3);
		rotate: 180deg;
		transition:
			color 120ms var(--ease-out),
			translate 120ms var(--ease-out);
	}

	/* ---- detail ---- */
	.head {
		display: flex;
		align-items: flex-start;
		gap: 8px;
		margin-top: -4px;
	}
	.back {
		flex: none;
		margin-left: -8px;
	}
	.title-block {
		flex: 1;
		min-width: 0;
		display: grid;
		gap: 6px;
		padding-top: 4px;
	}
	.pl-name {
		margin: 0;
		font-size: 1.375rem;
		font-weight: 600;
		letter-spacing: -0.02em;
		line-height: 1.1;
	}
	.name-btn {
		display: inline-flex;
		align-items: center;
		gap: 8px;
		max-width: 100%;
		text-align: left;
		color: var(--ink);
	}
	.name-btn span {
		overflow-wrap: anywhere;
	}
	.name-btn :global(.cb-icon) {
		flex: none;
		color: var(--ink-3);
		opacity: 0;
		transition: opacity 120ms var(--ease-out);
	}
	.name-btn:hover :global(.cb-icon),
	.name-btn:focus-visible :global(.cb-icon) {
		opacity: 1;
	}
	@media (pointer: coarse) {
		.name-btn :global(.cb-icon) {
			opacity: 1;
		}
	}
	.rename {
		width: 100%;
		height: 36px;
		padding: 0 10px;
		border: 1px solid var(--ink);
		border-radius: var(--r-md);
		background: var(--key);
		outline: none;
		font-size: 1.125rem;
		font-weight: 600;
		color: var(--ink);
	}
	.warn {
		font-size: 0.75rem;
		color: var(--ink-2);
	}
	.meta .by {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.keys {
		display: flex;
		align-items: center;
		gap: 10px;
		margin: 16px 0 20px;
	}
	/* Queue all: the graphite key (like Add); Shuffle: a key cap. */
	.go,
	.shuffle {
		display: inline-flex;
		align-items: center;
		gap: 8px;
		height: 44px;
		padding: 0 16px;
		font-size: 0.875rem;
		font-weight: 600;
	}
	.go {
		border-radius: var(--r-sm);
		background: var(--ink);
		color: var(--surface);
		box-shadow: 0 1px 0 color-mix(in oklab, var(--ink) 60%, #000);
		transition: translate 60ms linear;
	}
	.go:active:not(:disabled) {
		translate: 0 1px;
	}
	.go:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	.trash {
		margin-left: auto;
	}
	@media (hover: hover) {
		.trash:hover {
			color: var(--danger) !important;
		}
	}

	.tracks {
		list-style: none;
		margin: 0;
		padding: 0;
		border-top: 1px solid var(--line);
	}
	.row {
		border-bottom: 1px solid var(--line);
	}
	.body {
		display: flex;
		align-items: center;
		gap: 14px;
		min-height: 52px;
		padding: 8px 4px 8px 0;
		background: var(--bg);
		transition: background-color 120ms var(--ease-out);
	}
	.body .idx {
		position: relative;
		flex: none;
		width: 2.4em;
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
	.name {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-weight: 500;
		font-size: 0.9375rem;
	}
	.body .dur {
		flex: none;
		min-width: 3.4em;
	}
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
	@media (max-width: 560px) {
		.pl-row {
			grid-template-columns: 1.8em minmax(0, 1fr) auto auto;
			gap: 10px;
		}
		.pl-row .n {
			display: none;
		}
		.body {
			gap: 10px;
		}
		.body .idx {
			width: 1.8em;
		}
		.body .dur {
			display: none;
		}
		.keys .go,
		.keys .shuffle {
			flex: 1;
			justify-content: center;
		}
	}
</style>
