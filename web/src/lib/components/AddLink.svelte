<script lang="ts">
	import { flip } from 'svelte/animate';
	import { fly, fade } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import type { Station } from '#lib/station.svelte.ts';
	import { CircleAlert, ListMusic, ListPlus } from '@lucide/svelte';
	import Icon from '#lib/icons/Icon.svelte';
	import Avatar from './Avatar.svelte';
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
		({ queued: 'Waiting its turn', downloading: 'Grabbing it', done: 'In the library', failed: 'No luck' })[status] ?? status;
</script>

<section aria-label="Add a track">
	<form class="add" class:invalid onsubmit={submit}>
		<label for="add-url" class="sr-only">YouTube link</label>
		<input
			id="add-url"
			type="url"
			inputmode="url"
			autocomplete="off"
			placeholder="Drop a YouTube link"
			bind:value={url}
			oninput={() => {
				invalid = false;
				if (station.addNote?.error) station.addNote = null;
			}}
		/>
		<button type="submit" disabled={!url.trim()}>
			<Icon name="plus" size={18} /> Add
		</button>
	</form>
	{#if invalid}
		<p class="hint" transition:fade={{ duration: dur(150) }}>That's not a YouTube link. Grab one from youtube.com or youtu.be.</p>
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
					<span class="state">{#if busy}<ListMusic size={14} />{:else}<Icon name="check" size={14} />{/if}</span>
					<span class="what">
						<span class="name">{row.title}</span>
						<span class="sub">
							<span>{#if row.current}Grabbing {row.current.title ?? 'the next one'}{:else if busy}Waiting its turn{:else}All in{/if}</span>
							{#if row.failed}<span class="err">{row.failed} failed</span>{/if}
						</span>
					</span>
					<span class="pct num">{row.done}/{row.total}</span>
					<Avatar name={row.added_by} size={22} />
					{#if row.queued}
						<button
							class="icon-btn cancel"
							aria-label="Cancel the rest of {row.title}"
							title="Cancel the rest ({row.queued} not started)"
							onclick={() => station.send({ type: 'cancel_playlist', playlist_id: row.id })}
						>
							<Icon name="close" size={16} />
						</button>
					{/if}
					{#if busy}
						<span class="bar"><span style:transform="scaleX({row.progress})"></span></span>
					{/if}
				{:else}
					{@const job = row.job}
					{@const list = hintFor(job)}
					<span class="state">
						{#if job.status === 'done'}<Icon name="check" size={14} />{:else if job.status === 'failed'}<CircleAlert
								size={14}
							/>{:else}<span class="spinner"></span>{/if}
					</span>
					<span class="what">
						<span class="name">{job.title ?? job.url}</span>
						<span class="sub">
							<span>{label(job.status)}</span>
							{#if job.error}<span class="err">{job.error}</span>{/if}
						</span>
					</span>
					{#if job.status === 'downloading'}<span class="pct num">{Math.round(job.progress * 100)}%</span>{/if}
					<Avatar name={job.added_by} size={22} />
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
	/* A cream slot stuck on the bog; its inner edge lights up berry when you're in it. */
	.add {
		display: flex;
		gap: 6px;
		padding: 6px;
		border-radius: 18px;
		background: var(--surface);
		color: var(--ink);
		box-shadow:
			inset 0 0 0 3px transparent,
			var(--sticker);
		transition: box-shadow 160ms var(--ease-out);
	}
	.add:focus-within {
		box-shadow:
			inset 0 0 0 3px var(--accent-line),
			var(--sticker);
	}
	.add.invalid {
		box-shadow:
			inset 0 0 0 3px var(--danger),
			var(--sticker);
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
	/* Sits on the bog, so it gets its own cream tag to stay readable. */
	.hint {
		display: flex;
		align-items: center;
		gap: 6px;
		width: fit-content;
		margin: 16px 0 0;
		padding: 6px 12px;
		border-radius: 10px;
		background: var(--surface);
		box-shadow: 0 4px 0 var(--shade);
		font-size: 0.85rem;
		font-weight: 550;
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
		margin: 18px 0 0;
		padding: 0;
		display: grid;
		gap: 10px;
	}
	.job {
		position: relative;
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 10px 14px;
		border-radius: var(--r-md);
		background: var(--surface);
		color: var(--ink);
		box-shadow: 0 4px 0 var(--shade);
		overflow: hidden;
		font-size: 0.88rem;
	}
	.pct {
		flex: none;
		font-family: var(--font-display);
		font-weight: 650;
		color: var(--ink-2);
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
		gap: 10px;
		min-width: 0;
		color: var(--ink-2);
		font-size: 0.8rem;
	}
	.err,
	.failed .sub {
		color: var(--danger);
	}
	.bar {
		position: absolute;
		left: 0;
		right: 0;
		bottom: 0;
		height: 4px;
	}
	.bar span {
		display: block;
		height: 100%;
		background: var(--accent);
		transform-origin: left;
		transition: transform 400ms var(--ease-out);
	}
</style>
