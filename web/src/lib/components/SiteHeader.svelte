<script lang="ts">
	import { resolve } from '$app/paths';

	type NavKey = 'home' | 'library' | 'about';

	let { active = 'home' }: { active?: NavKey } = $props();

	const links: { href: `/${'' | 'library' | 'about'}`; label: string; key: NavKey }[] = [
		{ href: '/', label: 'Extract', key: 'home' },
		{ href: '/library', label: 'Library', key: 'library' },
		{ href: '/about', label: 'About', key: 'about' }
	];
</script>

<header class="bg-[rgba(250,250,250,0.84)] backdrop-blur-sm">
	<div class="mx-auto max-w-6xl px-6 py-6">
		<div class="mb-4 flex items-center justify-between gap-6">
			<nav class="flex flex-wrap items-center gap-6 text-sm text-stone">
				{#each links as link (`${link.href}:${link.key}`)}
					<a
						aria-current={active === link.key ? 'page' : undefined}
						class={`border-b pb-0.5 ${
							active === link.key ? 'border-ink text-ink' : 'border-transparent hover:border-ink hover:text-ink'
						}`}
						href={resolve(link.href)}>
						{link.label}
					</a>
				{/each}
				<a
					class="border-b border-transparent pb-0.5 hover:border-ink hover:text-ink"
					href={resolve('/about#api-reference')}>
					API
				</a>
			</nav>
		</div>
		<div class="border-b-2 border-ink pb-6">
			<a class="block text-center" href={resolve('/')}>
				<span class="block text-5xl font-bold tracking-tight text-ink md:text-6xl">LECTITO</span>
				<span class="mt-2 block font-serif text-sm text-stone italic"> Extract readable content from any web page </span>
			</a>
		</div>
	</div>
</header>
