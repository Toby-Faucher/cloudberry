<script lang="ts">
	import type { Station } from '#lib/station.svelte.ts';
	import type { Track } from '#lib/types.ts';
	import Art from './Art.svelte';
	import Icon from '#lib/icons/Icon.svelte';
	import { pop } from '#lib/motion/motion.ts';

	/** Just played: a shelf of small crooked covers. Tap one to queue it again. */
	let { station }: { station: Station } = $props();
	// A track played twice shows once, at its latest play.
	const recent = $derived((station.state?.history ?? []).filter((t, i, a) => a.findIndex((x) => x.id === t.id) === i));
	const tilt = [-4, 3, -2, 4.5, -3.5, 2];
	let flashed = $state<string | null>(null);

	function again(t: Track, e: MouseEvent) {
		station.send({ type: 'enqueue', track_id: t.id });
		flashed = t.id;
		pop((e.currentTarget as HTMLElement).querySelector('.cover'), 0.15);
		setTimeout(() => flashed === t.id && (flashed = null), 1400);
	}
</script>

{#if recent.length}
	<section aria-labelledby="hist-h">
		<h2 class="section-title" id="hist-h">Just played</h2>
		<ul class="shelf">
			{#each recent as t, i (t.id)}
				<li style:--tilt="{tilt[i % tilt.length]}deg">
					<button class="sleeve" onclick={(e) => again(t, e)} aria-label="Queue {t.title} again" title="Queue it again">
						<span class="cover">
							<Art track={t} size={92} />
							{#if flashed === t.id}<span class="done"><Icon name="check" size={28} /></span>{/if}
						</span>
						<span class="cap">{t.title}</span>
					</button>
				</li>
			{/each}
		</ul>
	</section>
{/if}

<style>
	.shelf {
		list-style: none;
		margin: 0 -8px;
		padding: 10px 8px 14px;
		display: flex;
		gap: 18px;
		overflow-x: auto;
		scrollbar-width: thin;
	}
	li {
		flex: none;
		width: 104px;
	}
	.sleeve {
		display: grid;
		justify-items: center;
		gap: 10px;
		width: 100%;
		text-align: center;
		color: var(--on-bg);
	}
	.cover {
		position: relative;
		display: block;
		border-radius: 14px;
		overflow: hidden;
		rotate: var(--tilt);
		box-shadow:
			0 0 0 4px var(--die),
			0 6px 0 4px var(--shade);
		transition:
			rotate 200ms var(--ease-out),
			translate 200ms var(--ease-out);
	}
	.cover :global(.art) {
		border-radius: 0;
	}
	@media (hover: hover) {
		.sleeve:hover .cover {
			rotate: 0deg;
			translate: 0 -4px;
		}
	}
	.done {
		position: absolute;
		inset: 0;
		display: grid;
		place-items: center;
		background: color-mix(in oklab, var(--accent) 88%, transparent);
		color: var(--on-accent);
	}
	.cap {
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
		font-size: 0.8rem;
		font-weight: 600;
		line-height: 1.25;
	}
</style>
