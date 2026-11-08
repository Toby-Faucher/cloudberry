<script lang="ts">
	import { Popover } from 'bits-ui';
	import type { Snippet } from 'svelte';
	import Icon from '#lib/icons/Icon.svelte';
	import type { Station } from '#lib/station.svelte.ts';
	import type { Playlist } from '#lib/types.ts';
	import { enter } from '#lib/motion/motion.ts';
	import { filterPlaylists, NAME_MAX, nameTaken, validName } from '#lib/saved.ts';

	/**
	 * "+ playlist": a small drop-down panel listing the saved playlists, plus "New playlist…"
	 * (name field). `newOnly` skips the list (Save queue as playlist).
	 */
	let {
		station,
		ids,
		label,
		newOnly = false,
		defaultName = '',
		triggerClass = 'icon-btn',
		children
	}: {
		station: Station;
		ids: string[];
		/** Accessible name of the trigger, e.g. "Add Song A to a playlist". */
		label: string;
		newOnly?: boolean;
		defaultName?: string;
		triggerClass?: string;
		children?: Snippet;
	} = $props();

	let open = $state(false);
	let creating = $state(false);
	let query = $state('');
	let name = $state('');
	const shown = $derived(filterPlaylists(station.playlists, query));
	const taken = $derived(validName(name) && nameTaken(station.playlists, name));

	$effect(() => {
		if (open) {
			creating = newOnly;
			query = '';
			name = defaultName;
		}
	});

	function pick(p: Playlist) {
		station.addToPlaylist(p, ids);
		open = false;
	}

	function create(e: SubmitEvent) {
		e.preventDefault();
		if (!validName(name)) return;
		station.createPlaylist(name, ids);
		station.showToast(ids.length ? `Saved “${name.trim()}”, ${ids.length} song${ids.length === 1 ? '' : 's'}` : `Made “${name.trim()}”`);
		open = false;
	}

	const focusOnMount = (el: HTMLElement) => el.focus();
</script>

<Popover.Root bind:open>
	<Popover.Trigger class={triggerClass} aria-label={label} title={newOnly ? undefined : 'Add to a playlist'}>
		{#if children}{@render children()}{:else}<Icon name="list-plus" size={18} />{/if}
	</Popover.Trigger>
	<Popover.Portal>
		<Popover.Content class="pl-pop" sideOffset={6} align="end" collisionPadding={16}>
			<div class="panel" use:enter={{ y: [-6, 0], opacity: [0, 1], duration: 160, ease: 'outQuart' }}>
				{#if !creating}
					<p class="label head">Add to playlist</p>
					{#if station.playlists.length > 6}
						<input class="filter" bind:value={query} placeholder="Find a playlist" aria-label="Find a playlist" use:focusOnMount />
					{/if}
					<ul class="lists" aria-label="Playlists">
						{#each shown as p (p.id)}
							<li>
								<button class="opt" onclick={() => pick(p)}>
									<span class="nm">{p.name}</span>
									<span class="n num">{String(p.track_ids.length).padStart(2, '0')}</span>
								</button>
							</li>
						{:else}
							<li class="none">{station.playlists.length ? 'No match.' : 'No playlists yet.'}</li>
						{/each}
					</ul>
					<button class="opt new" onclick={() => (creating = true)}>
						<Icon name="plus" size={16} /><span>New playlist…</span>
					</button>
				{:else}
					<form onsubmit={create}>
						<label class="label head" for="pl-new-name">{newOnly ? 'Save as playlist' : 'New playlist'}</label>
						<input id="pl-new-name" class="filter" bind:value={name} maxlength={NAME_MAX} placeholder="Name it" autocomplete="off" use:focusOnMount />
						{#if taken}<p class="warn">There's already one called that. Fine, just saying.</p>{/if}
						<div class="row">
							{#if !newOnly}<button type="button" class="link" onclick={() => (creating = false)}>Back</button>{/if}
							<button type="submit" class="save" disabled={!validName(name)}>
								{ids.length ? `Save ${ids.length} song${ids.length === 1 ? '' : 's'}` : 'Create'}
							</button>
						</div>
					</form>
				{/if}
			</div>
		</Popover.Content>
	</Popover.Portal>
</Popover.Root>

<style>
	:global(.pl-pop) {
		z-index: 80;
		outline: none;
	}
	.panel {
		width: min(280px, calc(100vw - 32px));
		padding: 8px;
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: var(--r-md);
		box-shadow: var(--shadow-lift);
	}
	.head {
		margin: 4px 4px 10px;
	}
	.lists {
		list-style: none;
		margin: 0;
		padding: 0;
		max-height: 240px;
		overflow-y: auto;
	}
	.opt {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 10px;
		min-height: 36px;
		padding: 0 8px;
		border-radius: var(--r-sm);
		text-align: left;
		font-size: 0.875rem;
		font-weight: 500;
		color: var(--ink);
	}
	.opt:hover,
	.opt:focus-visible {
		background: var(--surface-2);
	}
	.nm {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.n {
		font-family: var(--font-mono);
		font-size: 0.6875rem;
		color: var(--ink-3);
	}
	.new {
		margin-top: 4px;
		border-top: 1px solid var(--line);
		border-radius: 0 0 var(--r-sm) var(--r-sm);
		color: var(--ink-2);
	}
	.none {
		padding: 6px 8px 8px;
		font-size: 0.8125rem;
		color: var(--ink-3);
	}
	.filter {
		width: 100%;
		height: 36px;
		margin-bottom: 6px;
		padding: 0 10px;
		border: 1px solid var(--line-2);
		border-radius: var(--r-md);
		background: var(--key);
		outline: none;
		font-size: 0.875rem;
		color: var(--ink);
	}
	.filter:focus {
		border-color: var(--ink);
	}
	.filter::placeholder {
		color: var(--ink-3);
	}
	.warn {
		margin: 0 4px 8px;
		font-size: 0.75rem;
		color: var(--ink-2);
	}
	.row {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 12px;
		margin-top: 4px;
	}
	.link {
		margin-right: auto;
		padding: 0 4px;
		font-size: 0.8125rem;
		color: var(--ink-2);
		text-decoration: underline;
		text-underline-offset: 3px;
	}
	.save {
		height: 36px;
		padding: 0 14px;
		border-radius: var(--r-sm);
		background: var(--ink);
		color: var(--surface);
		font-size: 0.8125rem;
		font-weight: 600;
	}
	.save:disabled {
		opacity: 0.4;
	}
</style>
