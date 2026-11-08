<script lang="ts" module>
	/** Shown in the help dialog. Add a row here when you bind a new key. */
	export const SHORTCUTS: [keys: string[], what: string][] = [
		[['Space'], 'Play / pause for everyone'],
		[['N'], 'Skip (with undo)'],
		[['/'], 'Chat'],
		[['?'], 'This help'],
		[['Ctrl', 'V'], 'Paste a YouTube link anywhere to add it']
	];
</script>

<script lang="ts">
	import { Dialog } from 'bits-ui';
	import { X } from '@lucide/svelte';
	import type { Station } from '#lib/station.svelte.ts';
	import { focusChat, ui } from '#lib/ui.svelte.ts';
	import { isTyping, isYouTubeUrl } from '#lib/util.ts';

	/** Global keyboard shortcuts, global paste-to-add, and the "?" help dialog. */
	let { station }: { station: Station } = $props();

	const mac = typeof navigator !== 'undefined' && /Mac|iP(hone|ad)/.test(navigator.platform);

	function onkeydown(e: KeyboardEvent) {
		if (e.defaultPrevented || e.ctrlKey || e.metaKey || e.altKey || isTyping(e.target)) return;
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
				station.skip();
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
		station.send({ type: 'add', url: text });
		station.showToast('Adding your link…');
	}
</script>

<svelte:window {onkeydown} {onpaste} />

<Dialog.Root bind:open={ui.help}>
	<Dialog.Portal>
		<Dialog.Overlay class="help-scrim" />
		<Dialog.Content class="help card">
			<div class="help-head">
				<Dialog.Title class="help-title">Keyboard shortcuts</Dialog.Title>
				<Dialog.Close class="icon-btn" aria-label="Close"><X size={18} /></Dialog.Close>
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
		background: rgb(20 12 6 / 0.35);
		animation: help-fade 180ms var(--ease-out);
	}
	:global(.help) {
		position: fixed;
		left: 50%;
		top: 50%;
		translate: -50% -50%;
		z-index: 81;
		width: min(420px, calc(100% - 32px));
		padding: 18px 20px 20px;
		outline: none;
		transform-origin: center;
		animation: help-in 220ms var(--ease-out);
	}
	:global(.help-head) {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 10px;
	}
	:global(.help-title) {
		font-family: var(--font-display);
		font-size: 1.15rem;
		font-weight: 600;
		letter-spacing: -0.02em;
	}
	.help-list {
		margin: 0;
		display: grid;
	}
	.help-row {
		display: flex;
		align-items: center;
		gap: 16px;
		padding: 9px 0;
		border-top: 1px solid var(--line);
	}
	dt {
		flex: none;
		min-width: 96px;
		display: flex;
		align-items: center;
		gap: 4px;
	}
	dd {
		margin: 0;
		color: var(--ink-2);
		font-size: 0.9rem;
	}
	kbd {
		display: inline-grid;
		place-items: center;
		min-width: 26px;
		height: 26px;
		padding: 0 7px;
		border-radius: 7px;
		font: 600 0.78rem var(--font-body);
		background: var(--surface-2);
		border: 1px solid var(--line);
		border-bottom-width: 2px;
	}
	.plus {
		color: var(--ink-3);
		font-size: 0.75rem;
	}
	.help-foot {
		margin: 12px 0 0;
		color: var(--ink-3);
		font-size: 0.8rem;
	}
	@keyframes help-fade {
		from {
			opacity: 0;
		}
	}
	@keyframes help-in {
		from {
			opacity: 0;
			transform: scale(0.96);
		}
	}
</style>
