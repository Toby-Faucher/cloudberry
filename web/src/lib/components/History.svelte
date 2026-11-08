<script lang="ts">
	import type { Station } from '#lib/station.svelte.ts';
	import Library from './Library.svelte';

	let { station }: { station: Station } = $props();
	// A track played twice shows once, at its latest play.
	const recent = $derived((station.state?.history ?? []).filter((t, i, a) => a.findIndex((x) => x.id === t.id) === i));
</script>

{#if recent.length}
	<Library {station} tracks={recent} title="Recently played" limit={4} />
{/if}
