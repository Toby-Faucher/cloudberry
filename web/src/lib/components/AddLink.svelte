<script lang="ts">
	import { flip } from 'svelte/animate';
	import { fly, fade } from 'svelte/transition';
	import { quartOut } from 'svelte/easing';
	import type { Station } from '#lib/station.svelte.ts';
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
		<label for="add-url" class="prefix label">Link</label>
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
			<Icon name="plus" size={16} /> Add
		</button>
	</form>
	{#if invalid}
		<p class="hint" transition:fade={{ duration: dur(150) }}>That's not a YouTube link. Grab one from youtube.com or youtu.be.</p>
	{:else if station.addNote}
		<p class="hint" class:note={!station.addNote.error} role={station.addNote.error ? 'alert' : 'status'} transition:fade={{ duration: dur(150) }}>
			{#if station.addNote.error}<Icon name="alert" size={14} />{:else}<span class="led on blink"></span>{/if}
			{station.addNote.text}
		</p>
	{/if}

	{#if rows.length}
		<ul class="jobs">
			{#each rows as row (row.key)}
				<li
					class="job {row.kind === 'job' ? row.job.status : row.queued || row.current ? 'downloading' : 'done'}"
					animate:flip={{ duration: dur(300) }}
					in:fly={{ y: -4, duration: dur(180), easing: quartOut }}
					out:fade={{ duration: dur(140) }}
				>
				{#if row.kind === 'playlist'}
					{@const busy = row.queued > 0 || !!row.current}
					<span class="state">{#if busy}<Icon name="list" size={16} />{:else}<Icon name="check" size={16} />{/if}</span>
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
							<Icon name="close" size={14} />
						</button>
					{/if}
					{#if busy}
						<span class="bar"><span style:transform="scaleX({row.progress})"></span></span>
					{/if}
				{:else}
					{@const job = row.job}
					{@const list = hintFor(job)}
					<span class="state">
						{#if job.status === 'done'}<Icon name="check" size={16} />{:else if job.status === 'failed'}<Icon
								name="alert"
								size={16}
							/>{:else}<span class="led on blink"></span>{/if}
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
							<Icon name="list-plus" size={16} /> Add the whole playlist?
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
	/* The input slot: a printed "Link" tag, the field, and a graphite Add key. */
	.add {
		display: flex;
		align-items: center;
		height: 52px;
		padding: 5px 5px 5px 0;
		border-radius: var(--r-md);
		background: var(--key);
		box-shadow: inset 0 0 0 1px var(--line-2);
		transition: box-shadow 120ms var(--ease-out);
	}
	.add:focus-within {
		box-shadow: inset 0 0 0 1px var(--ink);
	}
	.add.invalid {
		box-shadow: inset 0 0 0 1px var(--danger);
	}
	.prefix {
		align-self: stretch;
		display: grid;
		place-items: center;
		padding: 0 14px;
		margin-right: 4px;
		border-right: 1px solid var(--line);
		color: var(--ink-3);
	}
	input {
		flex: 1;
		min-width: 0;
		height: 100%;
		border: 0;
		background: transparent;
		padding: 0 8px;
		outline: none;
		font-size: 0.9375rem;
	}
	input::placeholder {
		color: var(--ink-3);
	}
	.add button {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		height: 100%;
		padding: 0 16px;
		border-radius: var(--r-sm);
		background: var(--ink);
		color: var(--surface);
		font-weight: 600;
		font-size: 0.875rem;
		transition:
			opacity 120ms var(--ease-out),
			background-color 120ms var(--ease-out);
	}
	.add button:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}
	.hint {
		display: flex;
		align-items: center;
		gap: 8px;
		margin: 10px 0 0;
		font-family: var(--font-mono);
		font-stretch: 87.5%;
		font-size: 0.75rem;
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
		gap: 6px;
		height: 32px;
		padding: 0 10px;
		border-radius: var(--r-md);
		box-shadow: inset 0 0 0 1px var(--line-2);
		color: var(--ink);
		font-weight: 550;
		font-size: 0.8125rem;
		white-space: nowrap;
	}
	@media (hover: hover) {
		.whole:hover {
			background: var(--surface-2);
		}
	}
	.cancel {
		flex: none;
		margin-left: auto;
	}

	.jobs {
		list-style: none;
		margin: 12px 0 0;
		padding: 0;
		border-top: 1px solid var(--line);
	}
	.job {
		position: relative;
		display: flex;
		align-items: center;
		gap: 12px;
		min-height: 52px;
		padding: 8px 0;
		border-bottom: 1px solid var(--line);
		font-size: 0.875rem;
	}
	.pct {
		flex: none;
		font-family: var(--font-mono);
		font-size: 0.75rem;
		color: var(--ink-2);
	}
	.state {
		display: grid;
		place-items: center;
		width: 20px;
		flex: none;
		color: var(--ink-2);
	}
	.failed .state {
		color: var(--danger);
	}
	.what {
		flex: 1;
		min-width: 0;
		display: grid;
		gap: 2px;
	}
	.name {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		font-weight: 550;
	}
	.sub {
		display: flex;
		gap: 10px;
		min-width: 0;
		font-family: var(--font-mono);
		font-stretch: 87.5%;
		font-size: 0.6875rem;
		color: var(--ink-2);
	}
	.err,
	.failed .sub {
		color: var(--danger);
	}
	.bar {
		position: absolute;
		left: 0;
		right: 0;
		bottom: -1px;
		height: 2px;
	}
	.bar span {
		display: block;
		height: 100%;
		background: var(--accent);
		transform-origin: left;
		transition: transform 300ms var(--ease-out);
	}
</style>
