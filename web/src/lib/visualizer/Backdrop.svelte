<script lang="ts">
	import Bloom from './Bloom.svelte';
	import { viz } from './state.svelte.ts';
</script>

<!-- Ambient app background. Place before the app's (positioned) root so it paints behind it. -->
{#if !viz.lean}
	<div class="backdrop" aria-hidden="true">
		<Bloom variant="fill" scale={0.35} />
	</div>
{/if}

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		pointer-events: none;
		opacity: 0.4;
		mask-image: radial-gradient(130% 90% at 50% 0%, black 20%, transparent 85%);
	}
</style>
