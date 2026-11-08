<script lang="ts">
	import { stickerFor } from '#lib/reactions.ts';

	/**
	 * A reaction drawn as a die-cut sticker: flat 2–3 tone art, a thick white outline and a hard
	 * drop under it. Fixed colours in both themes, like a real sticker. Decorative: callers label it.
	 */
	let { emoji, size = 40 }: { emoji: string; size?: number } = $props();
	const s = $derived(stickerFor(emoji));
	const INK = '#4a2508';
</script>

{#snippet art()}
	{#if s?.id === 'flame'}
		<path d="M24 4.5C31 11 39 18 39 29a15 15 0 0 1-30 0c0-7 4-11.5 7.5-16.5 1.5 4.5 3.5 7 6 7.5C21 14 21.5 9 24 4.5Z" fill="#e0553a" />
		<path d="M24 18c5 5 9 9 9 14.5a9 9 0 0 1-18 0c0-4.5 3-7 5-10 1.5 2.5 2.5 3.5 4 4-.5-3-.5-5.5 0-8.5Z" fill="#f0912b" />
		<path d="M24 30c2.5 2 4 4 4 6a4 4 0 0 1-8 0c0-2 2-4 4-6Z" fill="#fbc77a" />
	{:else if s?.id === 'laugh'}
		<path d="M8 22c-3.5 4.5-3.5 8.5 0 8.5s3.5-4 0-8.5ZM40 22c-3.5 4.5-3.5 8.5 0 8.5s3.5-4 0-8.5Z" fill="#6ec6e8" />
		<circle cx="24" cy="25" r="16.5" fill="#f7ae4a" />
		<ellipse cx="16.5" cy="14.5" rx="4" ry="2.4" transform="rotate(-32 16.5 14.5)" fill="#fbc77a" />
		<path d="M14.5 21q3.2-4.2 6.4 0M27.1 21q3.2-4.2 6.4 0" fill="none" stroke={INK} stroke-width="2.8" stroke-linecap="round" />
		<path d="M14.5 26.5h19a9.5 9.5 0 0 1-19 0Z" fill={INK} />
		<path d="M19.2 33.3a4.8 2.8 0 0 1 9.6 0 6.5 6.5 0 0 1-9.6 0Z" fill="#e0553a" />
	{:else if s?.id === 'heart'}
		<path d="M24 41.5C11 32.5 5.5 25.5 5.5 17.5A9.5 9.5 0 0 1 24 13a9.5 9.5 0 0 1 18.5 4.5c0 8-5.5 15-18.5 24Z" fill="#e0553a" />
		<path d="M42 21c-1.5 7-7 12.5-18 20.5 2-2.5 9.5-8.5 12.5-13 2.5-3.5 4.5-6 5.5-7.5Z" fill="#b8402a" />
		<ellipse cx="14" cy="18.5" rx="3.6" ry="2.3" transform="rotate(-35 14 18.5)" fill="#fffdf5" />
	{:else if s?.id === 'clap'}
		<path d="M8.5 8.5l3 3.5M5 16.5l4 .6M39.5 8.5l-3 3.5M43 16.5l-4 .6" fill="none" stroke="#e0553a" stroke-width="2.8" stroke-linecap="round" />
		<rect x="11.5" y="12" width="13" height="27" rx="6.5" transform="rotate(-16 18 25.5)" fill="#fbc77a" />
		<rect x="23" y="12" width="13" height="27" rx="6.5" transform="rotate(14 29.5 25.5)" fill="#f7ae4a" />
		<path d="M27.5 17.5l1.6 8M31.8 18.6l-.1 7.4" fill="none" stroke="#d9741a" stroke-width="2.2" stroke-linecap="round" />
	{:else if s?.id === 'party'}
		<path d="M7 41 17 14l17 17Z" fill="#f0912b" stroke="#f0912b" stroke-width="2" stroke-linejoin="round" />
		<path d="M13.6 23.3 22.4 19.4l3.9 3.9-14.6 6.4ZM10.4 31.8l18.8-5.9 3.2 3.2-23.4 8.4Z" fill="#e0553a" />
		<rect x="29" y="5.5" width="4.2" height="7.5" rx="1.6" transform="rotate(28 31 9)" fill="#7db53a" />
		<circle cx="40" cy="18" r="3" fill="#c8e04a" />
		<rect x="37" y="28" width="4.2" height="7.5" rx="1.6" transform="rotate(-38 39 32)" fill="#e0553a" />
		<circle cx="22" cy="7.5" r="2.5" fill="#fbc77a" />
	{:else if s?.id === 'zzz'}
		<circle cx="21.5" cy="28.5" r="15" fill="#fbc77a" />
		<ellipse cx="15" cy="20" rx="3.4" ry="2" transform="rotate(-30 15 20)" fill="#fffdf5" />
		<path d="M12.5 28q3 3 6 0M24.5 28q3 3 6 0" fill="none" stroke={INK} stroke-width="2.6" stroke-linecap="round" />
		<ellipse cx="21.5" cy="36" rx="2.6" ry="2" fill={INK} />
		<path d="M30 5h9.5L30 14.5h9.5M38.5 19.5h5l-5 5h5" fill="none" stroke="#7db53a" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round" />
	{/if}
{/snippet}

{#if s}
	<svg class="sticker" viewBox="-2 -2 52 54" width={size} height={size} aria-hidden="true">
		<g class="cut drop">{@render art()}</g>
		<g class="cut">{@render art()}</g>
		<g>{@render art()}</g>
	</svg>
{:else}
	<span class="glyph" style:font-size="{size * 0.8}px" aria-hidden="true">{emoji}</span>
{/if}

<style>
	.sticker {
		display: block;
		flex: none;
		overflow: visible;
	}
	/* The die-cut: every shape again, fat and white, under the art. */
	.cut :global(*) {
		fill: #fffdf5;
		stroke: #fffdf5;
		stroke-width: 7px;
		stroke-linejoin: round;
		stroke-linecap: round;
	}
	/* Its hard drop: same outline, nudged down, a darker flat tone. */
	.drop {
		translate: 0 2.5px;
	}
	.drop :global(*) {
		fill: rgb(30 40 20 / 0.35);
		stroke: rgb(30 40 20 / 0.35);
	}
	.glyph {
		line-height: 1;
	}
</style>
