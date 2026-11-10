<script lang="ts">
	import { Popover, RadioGroup } from 'bits-ui';
	import { tinted } from '#lib/logo.ts';
	import { ACCENTS, setAccent, ui, type AccentId } from '#lib/ui.svelte.ts';
	import Berry from './Berry.svelte';

	/**
	 * The header berry is a key: press it and the accent presets fan out around it in a quarter
	 * circle (it lives in the top-left corner, so the arc opens right and down).
	 */
	const BEAD = 7; // Berry footprint is 3 beads
	const HALF = (BEAD * 3) / 2;
	const R = 124; // arc radius to berry centres: 18° steps leave ~4px between 34px berries
	const pos = ACCENTS.map((_, i) => {
		const a = (i / (ACCENTS.length - 1)) * (Math.PI / 2);
		return [Math.cos(a) * R, Math.sin(a) * R];
	});

	let open = $state(false);
	// Each option is the logo itself in that colour (same recolour as the header mark).
	// Drawn up front (six 96px canvases) so the fan-out never shows empty keys.
	let logos = $state<Record<string, string>>({ berry: '/logo-mark.png' });
	for (const a of ACCENTS.slice(1)) tinted('/logo-mark.png', a.swatch).then((u) => (logos[a.id] = u), () => {});

	const label = $derived(ACCENTS.find((a) => a.id === ui.accent)!.label);
</script>

<Popover.Root bind:open>
	<Popover.Trigger class="dial-key" aria-label="Accent colour: {label}" title="Accent colour">
		<Berry bead={BEAD} />
	</Popover.Trigger>
	<Popover.Portal>
		<!-- A 0×0 box pinned to the berry's centre; everything below is placed from that point. -->
		<Popover.Content
			class="dial"
			side="bottom"
			align="start"
			sideOffset={-HALF}
			alignOffset={HALF}
			avoidCollisions={false}
			aria-label="Accent colour"
		>
			<RadioGroup.Root
				class="ring"
				value={ui.accent}
				onValueChange={(v) => setAccent(v as AccentId)}
				aria-label="Accent colour"
			>
				{#each ACCENTS as a, i (a.id)}
					<RadioGroup.Item
						value={a.id}
						class="sw"
						aria-label={a.label}
						title={a.label}
						style="--x: {pos[i][0]}px; --y: {pos[i][1]}px; --i: {i}"
						onclick={() => (open = false)}
					>
						{#if logos[a.id]}<img src={logos[a.id]} alt="" width="96" height="96" draggable="false" />{/if}
					</RadioGroup.Item>
				{/each}
			</RadioGroup.Root>
		</Popover.Content>
	</Popover.Portal>
</Popover.Root>

<style>
	:global(.dial-key) {
		display: grid;
		padding: 0;
		border-radius: 50%;
		transition: scale 120ms var(--ease-out);
	}
	:global(.dial-key:active) {
		scale: 0.92;
	}
	:global(.dial) {
		z-index: 80;
		width: 0;
		height: 0;
		outline: none;
	}
	:global(.dial .sw) {
		position: absolute;
		left: -17px;
		top: -17px;
		width: 34px;
		height: 34px;
		display: grid;
		place-items: center;
		padding: 0;
		border-radius: 50%;
		/* The logo already has its white sticker rim; a soft shadow lifts it off the page. */
		filter: drop-shadow(0 3px 6px rgb(0 0 0 / 0.3));
		transition: scale 120ms var(--ease-out);
		translate: var(--x) var(--y);
		animation: fan 220ms var(--ease-out) calc(var(--i) * 18ms) both;
	}
	:global(.dial .sw img) {
		width: 100%;
		height: 100%;
		object-fit: contain;
		pointer-events: none;
	}
	:global(.dial .sw:hover) {
		scale: 1.12;
	}
	:global(.dial .sw[data-state='checked']) {
		outline: 2px solid var(--ink);
		outline-offset: 4px;
	}
	:global(.dial .sw:focus-visible) {
		outline: 2px solid var(--ink);
		outline-offset: 4px;
	}
	@keyframes fan {
		from {
			opacity: 0;
			translate: 0 0;
			scale: 0.4;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		:global(.dial .sw) {
			animation: none;
		}
	}
</style>
