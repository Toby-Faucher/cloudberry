<script lang="ts">
	import type { Mode } from './state.svelte.ts';
	import Halo from './Halo.svelte';
	import ShaderCanvas from './ShaderCanvas.svelte';
	import Bars from './Bars.svelte';

	/**
	 * halo/bloom draw around their positioned parent (the art's container) and bleed
	 * past it; bars is a normal block strip.
	 */
	let { mode }: { mode: Mode } = $props();
</script>

{#if mode === 'halo'}
	<Halo />
{:else if mode === 'bloom'}
	<ShaderCanvas variant="aura" />
{:else}
	<Bars />
{/if}
