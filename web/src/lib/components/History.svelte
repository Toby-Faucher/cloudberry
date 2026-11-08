<script lang="ts">
	import type { Station } from '#lib/station.svelte.ts';
	import type { Track } from '#lib/types.ts';
	import Art from './Art.svelte';
	import Icon from '#lib/icons/Icon.svelte';
	import { pop } from '#lib/motion/motion.ts';

	/** Just played: a row of square covers, newest first. Tap one to queue it again. */
	let { station }: { station: Station } = $props();
	// A track played twice shows once, at its latest play.
	const recent = $derived((station.state?.history ?? []).filter((t, i, a) => a.findIndex((x) => x.id === t.id) === i));

	let flashed = $state<string | null>(null);

	function again(t: Track, e: MouseEvent) {
		station.send({ type: 'enqueue', track_id: t.id });
		flashed = t.id;
		pop((e.currentTarget as HTMLElement).querySelector('.cover'), 0.05);
		setTimeout(() => flashed === t.id && (flashed = null), 1400);
	}
</script>

{#if recent.length}
	<section aria-labelledby="hist-h">
		<h2 class="section-title" id="hist-h">Just played <span class="count num">{String(recent.length).padStart(2, '0')}</span></h2>
		<ul class="shelf">
			{#each recent as t, i (t.id)}
				<li>
					<button class="sleeve" onclick={(e) => again(t, e)} aria-label="Queue {t.title} again" title="Queue it again">
						<span class="cover">
							<Art track={t} size={96} />
							{#if flashed === t.id}<span class="done"><Icon name="check" size={24} /><span class="label">Queued</span></span>{/if}
						</span>
						<span class="cap"><span class="i num">{String(i + 1).padStart(2, '0')}</span>{t.title}</span>
					</button>
				</li>
			{/each}
		</ul>
	</section>
{/if}

<style>
	.shelf {
		list-style: none;
		margin: 0;
		padding: 0 0 6px;
		display: flex;
		gap: 16px;
		overflow-x: auto;
		scrollbar-width: thin;
	}
	li {
		flex: none;
		width: 96px;
	}
	.sleeve {
		display: grid;
		gap: 8px;
		width: 100%;
		text-align: left;
		color: var(--ink);
		border-radius: var(--r-sm);
	}
	.cover {
		position: relative;
		display: block;
		border-radius: var(--r-sm);
		overflow: hidden;
		transition: translate 120ms var(--ease-out);
	}
	@media (hover: hover) {
		.sleeve:hover .cover {
			translate: 0 -2px;
		}
		.sleeve:hover .cap {
			color: var(--ink);
		}
	}
	.done {
		position: absolute;
		inset: 0;
		display: grid;
		place-content: center;
		justify-items: center;
		gap: 6px;
		background: var(--accent);
		color: var(--on-accent);
	}
	.done .label {
		color: var(--on-accent);
	}
	.cap {
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
		font-size: 0.75rem;
		font-weight: 500;
		line-height: 1.3;
		color: var(--ink-2);
	}
	.i {
		margin-right: 6px;
		font-family: var(--font-mono);
		font-size: 0.625rem;
		color: var(--ink-3);
	}
</style>
