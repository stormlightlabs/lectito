<script lang="ts">
	import favicon from '$lib/assets/favicon.svg';
	import SiteFooter from '$lib/components/SiteFooter.svelte';
	import './layout.css';

	let { children } = $props();
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>

<div class="min-h-screen">
	{@render children()}
	<SiteFooter />
</div>
