<script lang="ts">
	import { flip } from 'svelte/animate';
	import { fly, fade } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import type { Station } from '#lib/station.svelte.ts';
	import { CircleAlert, Check, ListMusic, ListPlus, Plus, X } from '@lucide/svelte';
	import { dur, isYouTubeUrl } from '#lib/util.ts';
	import { groupJobs, jobVideo, playlistUrl } from '#lib/playlist.ts';
	import type { Job } from '#lib/types.ts';

	let { station }: { station: Station } = $props();
	let url = $state('');
	let invalid = $state(false);

	function submit(e: SubmitEvent) {
		e.preventDefault();
		const v = url.trim();
		if (!v) return;
		if (!isYouTubeUrl(v)) {
			invalid = true;
			return;
		}
		station.add(v);
		url = '';
	}

	const rows = $derived(groupJobs(station.jobs));
	/** The playlist our job's video was added from (`watch?v=…&list=…`), if any. */
	const hintFor = (job: Job) => (job.added_by === station.name ? station.listHints[jobVideo(job) ?? ''] : undefined);
	function addWhole(job: Job, list: string) {
		delete station.listHints[jobVideo(job) ?? ''];
		station.add(playlistUrl(list));
	}

	const label = (status: string) =>
		({ queued: 'Waiting', downloading: 'Downloading', done: 'Added', failed: 'Failed' })[status] ?? status;
</script>

<section aria-label="Add a track">
	<form class="add" class:invalid onsubmit={submit}>
		<label for="add-url" class="sr-only">YouTube link</label>
		<input
			id="add-url"
			type="url"
			inputmode="url"
			autocomplete="off"
			placeholder="Paste a YouTube link"
			bind:value={url}
			oninput={() => {
				invalid = false;
				if (station.addNote?.error) station.addNote = null;
			}}
		/>
		<button type="submit" disabled={!url.trim()}>
			<Plus size={16} /> Add
		</button>
	</form>
	{#if invalid}
		<p class="hint" transition:fade={{ duration: dur(150) }}>That isn't a YouTube link. Try one from youtube.com or youtu.be.</p>
	{:else if station.addNote}
		<p class="hint" class:note={!station.addNote.error} role={station.addNote.error ? 'alert' : 'status'} transition:fade={{ duration: dur(150) }}>
			{#if station.addNote.error}<CircleAlert size={14} />{:else}<span class="spinner"></span>{/if}
			{station.addNote.text}
		</p>
	{/if}

	{#if rows.length}
		<ul class="jobs">
			{#each rows as row (row.key)}
				<li
					class="job {row.kind === 'job' ? row.job.status : row.queued || row.current ? 'downloading' : 'done'}"
					animate:flip={{ duration: dur(300) }}
					in:fly={{ y: -6, duration: dur(260), easing: cubicOut }}
					out:fade={{ duration: dur(200) }}
				>
				{#if row.kind === 'playlist'}
					{@const busy = row.queued > 0 || !!row.current}
					<span class="state">{#if busy}<ListMusic size={14} />{:else}<Check size={14} strokeWidth={2.5} />{/if}</span>
					<span class="what">
						<span class="name">{row.title} · <span class="num">{row.done}/{row.total}</span></span>
						<span class="sub">
							{#if row.current}Downloading {row.current.title ?? 'next song'}{:else if busy}Waiting{:else}Done{/if}
							{#if row.failed}· {row.failed} failed{/if}
							· {row.added_by}
						</span>
					</span>
					{#if row.queued}
						<button
							class="icon-btn cancel"
							aria-label="Cancel the rest of {row.title}"
							title="Cancel the rest ({row.queued} not started)"
							onclick={() => station.send({ type: 'cancel_playlist', playlist_id: row.id })}
						>
							<X size={16} />
						</button>
					{/if}
					{#if busy}
						<span class="bar"><span style:transform="scaleX({row.progress})"></span></span>
					{/if}
				{:else}
					{@const job = row.job}
					{@const list = hintFor(job)}
					<span class="state">
						{#if job.status === 'done'}<Check size={14} strokeWidth={2.5} />{:else if job.status === 'failed'}<CircleAlert
								size={14}
							/>{:else}<span class="spinner"></span>{/if}
					</span>
					<span class="what">
						<span class="name">{job.title ?? job.url}</span>
						<span class="sub">
							{label(job.status)}{#if job.status === 'downloading'}&nbsp;· <span class="num">{Math.round(job.progress * 100)}%</span>{/if}
							{#if job.error}· {job.error}{/if}
							· {job.added_by}
						</span>
					</span>
					{#if list}
						<button class="whole" onclick={() => addWhole(job, list)}>
							<ListPlus size={15} /> Add the whole playlist?
						</button>
					{/if}
					{#if job.status === 'downloading' || job.status === 'queued'}
						<span class="bar"><span style:transform="scaleX({job.progress})"></span></span>
					{/if}
				{/if}
				</li>
			{/each}
		</ul>
	{/if}
</section>

<style>
	/* A flat slot with a thick edge that lights up berry when you're in it. */
	.add {
		display: flex;
		gap: 6px;
		padding: 6px;
		border-radius: 18px;
		background: var(--surface);
		border: 3px solid var(--line);
		transition: border-color 160ms var(--ease-out);
	}
	.add:focus-within {
		border-color: var(--accent-line);
	}
	.add.invalid {
		border-color: var(--danger);
	}
	input {
		flex: 1;
		min-width: 0;
		border: 0;
		background: transparent;
		padding: 10px 12px;
		outline: none;
		font-size: 1rem;
	}
	input::placeholder {
		color: var(--ink-3);
	}
	.add button {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		min-height: 46px;
		padding: 0 18px;
		border-radius: 12px;
		background: var(--accent);
		color: var(--on-accent);
		font-family: var(--font-display);
		font-size: 1.05rem;
		font-weight: 650;
		box-shadow: var(--lip);
		transition:
			opacity 160ms var(--ease-out),
			background-color 160ms var(--ease-out);
	}
	.add button:hover:not(:disabled) {
		background: var(--accent-hover);
	}
	.add button:disabled {
		opacity: 0.4;
		cursor: not-allowed;
		box-shadow: none;
	}
	.hint {
		display: flex;
		align-items: center;
		gap: 6px;
		margin: 8px 6px 0;
		font-size: 0.85rem;
		color: var(--danger);
	}
	.hint.note {
		color: var(--ink-2);
	}
	.whole {
		flex: none;
		margin-left: auto;
		display: inline-flex;
		align-items: center;
		gap: 5px;
		min-height: 36px;
		padding: 0 12px;
		border-radius: 10px;
		background: var(--accent-soft);
		color: var(--accent-ink);
		font-weight: 600;
		font-size: 0.82rem;
		white-space: nowrap;
	}
	.whole:hover {
		background: var(--accent);
		color: var(--on-accent);
	}
	.cancel {
		flex: none;
		margin-left: auto;
	}

	.jobs {
		list-style: none;
		margin: 10px 0 0;
		padding: 0;
		display: grid;
		gap: 6px;
	}
	.job {
		position: relative;
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 10px 14px;
		border-radius: var(--r-md);
		background: var(--surface-2);
		overflow: hidden;
		font-size: 0.88rem;
	}
	.state {
		display: grid;
		place-items: center;
		width: 22px;
		height: 22px;
		border-radius: 50%;
		flex: none;
		color: var(--accent-ink);
		background: var(--accent-soft);
	}
	.failed .state {
		color: var(--danger);
		background: color-mix(in oklab, var(--danger) 14%, transparent);
	}
	.spinner {
		width: 12px;
		height: 12px;
		border-radius: 50%;
		border: 2px solid currentColor;
		border-right-color: transparent;
		animation: spin 0.8s linear infinite;
	}
	@keyframes spin {
		to {
			transform: rotate(1turn);
		}
	}
	.what {
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
		color: var(--ink-2);
		font-size: 0.8rem;
	}
	.failed .sub {
		color: var(--danger);
	}
	.bar {
		position: absolute;
		left: 0;
		right: 0;
		bottom: 0;
		height: 2px;
	}
	.bar span {
		display: block;
		height: 100%;
		background: var(--accent);
		transform-origin: left;
		transition: transform 400ms var(--ease-out);
	}
</style>
