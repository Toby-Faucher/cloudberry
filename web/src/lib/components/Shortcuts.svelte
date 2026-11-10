<script lang="ts" module>
	/** Shown in the help dialog. Add a row here when you bind a new key. */
	export const SHORTCUTS: [keys: string[], what: string][] = [
		[['Space'], 'Play / pause for everyone'],
		[['N'], 'Hold to skip (with undo)'],
		[['M'], 'Mute / unmute (only for you)'],
		[['/'], 'Chat'],
		[['?'], 'This help'],
		[['F'], 'Lean-back (fullscreen visualizer)'],
		[['← →'], 'Lean-back: previous / next shader'],
		[['A'], 'Lean-back: new shader every track (auto-cycle)'],
		[['Ctrl', 'V'], 'Paste a YouTube link anywhere to add it']
	];
</script>

<script lang="ts">
	import { Dialog } from 'bits-ui';
	import Icon from '#lib/icons/Icon.svelte';
	import type { Station } from '#lib/station.svelte.ts';
	import { focusChat, toggleMute, ui } from '#lib/ui.svelte.ts';
	import { isTyping, isYouTubeUrl } from '#lib/util.ts';
	import { linkParts } from '#lib/playlist.ts';
	import { pressSkip, releaseSkip } from '#lib/hold.svelte.ts';
	import { konami } from '#lib/game.ts';
	import { startBerryRain } from '#lib/motion/smear.ts';

	/** Global keyboard shortcuts, global paste-to-add, and the "?" help dialog. */
	let { station }: { station: Station } = $props();

	const mac = typeof navigator !== 'undefined' && /Mac|iP(hone|ad)/.test(navigator.platform);

	const code = konami();

	function onkeydown(e: KeyboardEvent) {
		if (e.ctrlKey || e.metaKey || e.altKey || isTyping(e.target)) return;
		// ↑↑↓↓←→←→BA: local-only cloudberry rain.
		if (!e.repeat && code(e.key)) {
			const c = document.getElementById('berry-rain');
			if (c instanceof HTMLCanvasElement) void startBerryRain(c);
		}
		if (e.defaultPrevented) return;
		if (ui.help || ui.drawer) return; // a dialog owns the keyboard
		const t = e.target as HTMLElement;
		switch (e.key) {
			case ' ':
				// Space on a focused button/slider/draggable row is that control's own action.
				if (t !== document.body && t.closest('button, a, [role="slider"], [tabindex]')) return;
				e.preventDefault();
				station.togglePlay();
				break;
			case 'n':
			case 'N':
				// Like the button: hold to skip, a tap only shows the hint.
				if (!e.repeat) pressSkip(station);
				break;
			case 'm':
			case 'M':
				if (!e.repeat) toggleMute();
				break;
			case '/':
				e.preventDefault(); // don't type the slash into the input we're about to focus
				focusChat();
				break;
			case '?':
				ui.help = true;
				break;
		}
	}

	function onpaste(e: ClipboardEvent) {
		if (isTyping(e.target)) return;
		const text = e.clipboardData?.getData('text/plain')?.trim() ?? '';
		if (!isYouTubeUrl(text)) return;
		e.preventDefault();
		station.add(text);
		station.showToast(linkParts(text).playlist ? 'Loading the playlist…' : 'Adding your link…');
	}
</script>

<svelte:window {onkeydown} onkeyup={(e) => (e.key === 'n' || e.key === 'N') && releaseSkip()} onblur={releaseSkip} {onpaste} />

<Dialog.Root bind:open={ui.help}>
	<Dialog.Portal>
		<Dialog.Overlay class="help-scrim" />
		<Dialog.Content class="help card">
			<div class="help-head">
				<Dialog.Title class="help-title">Keyboard shortcuts</Dialog.Title>
				<Dialog.Close class="icon-btn" aria-label="Close"><Icon name="close" size={18} /></Dialog.Close>
			</div>
			<Dialog.Description class="sr-only">Shortcuts work anywhere except while typing.</Dialog.Description>
			<dl class="help-list">
				{#each SHORTCUTS as [keys, what] (what)}
					<div class="help-row">
						<dt>
							{#each keys as k, i (k)}
								{#if i}<span class="plus">+</span>{/if}<kbd>{k === 'Ctrl' && mac ? '⌘' : k}</kbd>
							{/each}
						</dt>
						<dd>{what}</dd>
					</div>
				{/each}
			</dl>
			<p class="help-foot">Shortcuts are ignored while you're typing.</p>
		</Dialog.Content>
	</Dialog.Portal>
</Dialog.Root>

<style>
	:global(.help-scrim) {
		position: fixed;
		inset: 0;
		z-index: 80;
		background: rgb(0 0 0 / 0.32);
		animation: help-fade 140ms var(--ease-out);
	}
	:global(.help) {
		position: fixed;
		left: 50%;
		top: 50%;
		translate: -50% -50%;
		z-index: 81;
		width: min(420px, calc(100% - 32px));
		padding: 0 16px 16px;
		outline: none;
		box-shadow: var(--shadow-lift);
		animation: help-in 160ms var(--ease-out);
	}
	:global(.help-head) {
		display: flex;
		align-items: center;
		justify-content: space-between;
		height: 52px;
		margin: 0 -6px 0 0;
	}
	:global(.help-title) {
		font-family: var(--font-mono);
		font-stretch: 87.5%;
		font-size: 0.6875rem;
		font-weight: 500;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--ink-2);
	}
	.help-list {
		margin: 0;
		display: grid;
	}
	.help-row {
		display: flex;
		align-items: center;
		gap: 16px;
		padding: 10px 0;
		border-top: 1px solid var(--line);
	}
	dt {
		flex: none;
		min-width: 104px;
		display: flex;
		align-items: center;
		gap: 4px;
	}
	dd {
		margin: 0;
		color: var(--ink);
		font-size: 0.875rem;
	}
	/* Key caps: the same off-white key with a lower lip as the transport. */
	kbd {
		display: inline-grid;
		place-items: center;
		min-width: 28px;
		height: 28px;
		padding: 0 7px;
		border-radius: var(--r-md);
		font: 500 0.6875rem var(--font-mono);
		background: var(--key);
		box-shadow:
			inset 0 0 0 1px var(--line-2),
			0 1px 0 var(--key-edge);
	}
	.plus {
		color: var(--ink-3);
		font-size: 0.75rem;
	}
	.help-foot {
		margin: 12px 0 0;
		padding-top: 12px;
		border-top: 1px solid var(--line);
		font-family: var(--font-mono);
		font-stretch: 87.5%;
		color: var(--ink-2);
		font-size: 0.6875rem;
	}
	@keyframes help-fade {
		from {
			opacity: 0;
		}
	}
	@keyframes help-in {
		from {
			opacity: 0;
			translate: -50% calc(-50% + 6px);
		}
	}
</style>
