<script lang="ts">
	import { resolve } from '$app/paths';
	import { SITE } from '$lib/content';
</script>

<footer class="mt-20 border-t-2 border-ink bg-[rgba(255,255,255,0.88)]">
	<div class="mx-auto max-w-6xl px-6 py-10">
		<div class="grid gap-8 md:grid-cols-4">
			<div>
				<h2 class="mb-4 text-lg font-bold tracking-tight text-ink">{SITE.name}</h2>
				<p class="font-serif text-sm text-stone">{SITE.footerTagline}</p>
			</div>
			<div>
				<p class="muted-label mb-4">Product</p>
				<div class="space-y-2 text-sm">
					<a class="block font-serif hover:text-stone" href={resolve('/')}>Extract</a>
					<a class="block font-serif hover:text-stone" href={resolve('/library')}>Library</a>
				</div>
			</div>
			<div>
				<p class="muted-label mb-4">Developers</p>
				<div class="space-y-2 text-sm">
					<a class="block font-serif hover:text-stone" href={resolve('/about#api-reference')}>API Reference</a>
					<a class="block font-serif hover:text-stone" href={resolve('/about#example-usage')}>Example Usage</a>
				</div>
			</div>
			<div>
				<p class="muted-label mb-4">Status</p>
				<div class="flex items-center gap-3 font-mono text-xs text-fog">
					<span class="site-status-dot h-2.5 w-2.5 rounded-full"></span>
					<span>{SITE.status}</span>
				</div>
			</div>
		</div>
		<div
			class="mt-10 flex flex-col gap-4 border-t border-mist pt-6 text-xs text-fog md:flex-row md:items-center md:justify-between">
			<p class="font-serif">{SITE.copyright}</p>
			<div class="flex gap-6 text-sm text-stone">
				<a class="font-serif hover:text-ink" href={resolve('/')}>Home</a>
				<a class="font-serif hover:text-ink" href={resolve('/library')}>Library</a>
				<a class="font-serif hover:text-ink" href={resolve('/about')}>About</a>
			</div>
		</div>
	</div>
</footer>
