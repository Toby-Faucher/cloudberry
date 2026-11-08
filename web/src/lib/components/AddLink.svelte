<script lang="ts">
	import { flip } from 'svelte/animate';
	import { fly, fade } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import type { Station } from '#lib/station.svelte.ts';
	import { CircleAlert, Check, Plus } from '@lucide/svelte';
	import { dur, isYouTubeUrl } from '#lib/util.ts';

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
		station.send({ type: 'add', url: v });
		url = '';
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
			oninput={() => (invalid = false)}
		/>
		<button type="submit" disabled={!url.trim()}>
			<Plus size={16} /> Add
		</button>
	</form>
	{#if invalid}
		<p class="hint" transition:fade={{ duration: dur(150) }}>That isn't a YouTube link. Try one from youtube.com or youtu.be.</p>
	{/if}

	{#if station.jobs.length}
		<ul class="jobs">
			{#each station.jobs as job (job.id)}
				<li
					class="job {job.status}"
					animate:flip={{ duration: dur(300) }}
					in:fly={{ y: -6, duration: dur(260), easing: cubicOut }}
					out:fade={{ duration: dur(200) }}
				>
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
					{#if job.status === 'downloading' || job.status === 'queued'}
						<span class="bar"><span style:transform="scaleX({job.progress})"></span></span>
					{/if}
				</li>
			{/each}
		</ul>
	{/if}
</section>

<style>
	.add {
		display: flex;
		gap: 6px;
		padding: 6px;
		border-radius: 18px;
		background: var(--surface);
		border: 1px solid var(--line);
		box-shadow: var(--shadow);
		transition:
			border-color 160ms var(--ease-out),
			box-shadow 160ms var(--ease-out);
	}
	.add:focus-within {
		border-color: color-mix(in oklab, var(--accent) 55%, var(--line));
		box-shadow:
			0 0 0 4px color-mix(in oklab, var(--accent) 14%, transparent),
			var(--shadow);
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
	}
	input::placeholder {
		color: var(--ink-3);
	}
	button {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 0 16px;
		border-radius: 12px;
		background: var(--ink);
		color: var(--bg);
		font-weight: 500;
		transition:
			opacity 160ms var(--ease-out),
			transform 120ms var(--ease-out);
	}
	button:active:not(:disabled) {
		transform: scale(0.96);
	}
	button:disabled {
		opacity: 0.25;
		cursor: default;
	}
	.hint {
		margin: 8px 6px 0;
		font-size: 0.85rem;
		color: var(--danger);
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
