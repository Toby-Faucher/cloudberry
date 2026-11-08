<script lang="ts">
	import '../app.css';
	import { Tooltip } from 'bits-ui';
	import type { LayoutProps } from './$types';

	let { children }: LayoutProps = $props();
</script>

<Tooltip.Provider delayDuration={200}>
	{@render children()}
</Tooltip.Provider>
