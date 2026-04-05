<script lang="ts">
	import { resolve } from '$app/paths';
	import SiteFooter from '$lib/components/SiteFooter.svelte';
	import SiteHeader from '$lib/components/SiteHeader.svelte';
	import type { LibraryResponse, LibrarySort } from '$lib/types';
	import { formatDate, formatHoursFromMinutes, formatNumber, formatReadingTime, formatWordCount } from '$lib/utils';

	type LibraryHref = '/library' | `/library?${string}`;
	type TQuery = {
		page: number;
		per_page: number;
		sort: LibrarySort;
		q: string;
		domain: string;
		date_from: string;
		date_to: string;
	};
	let {
		library,
		query,
		error = null,
		buildLink
	}: {
		library: LibraryResponse | null;
		query: TQuery;
		error?: string | null;
		buildLink: (overrides: Record<string, string | number | null | undefined>) => LibraryHref;
	} = $props();

	const SORT_OPTIONS: { label: string; value: LibrarySort }[] = [
		{ label: 'Recent', value: 'recent' },
		{ label: 'Popular', value: 'popular' },
		{ label: 'A–Z', value: 'alpha' }
	];

	const topDomains = $derived.by(() => library?.stats.top_domains ?? []);
	const domainOptions = $derived.by(() =>
		Array.from(
			new Set([...topDomains.map((entry) => entry.domain), ...(library?.items ?? []).map((item) => item.domain)])
		).sort()
	);
	const totalPages = $derived.by(() => (library ? Math.max(1, Math.ceil(library.total / library.per_page)) : 1));
	const hasResults = $derived.by(() => Boolean(library?.items?.length));
</script>

<svelte:head>
	<title>Library · Lectito</title>
	<meta name="description" content="Browse cached Lectito extractions by recency, popularity, or title." />
</svelte:head>

<div class="min-h-screen">
	<SiteHeader active="library" backHref="/" backLabel="← Back to Extract" />

	<div class="mx-auto max-w-6xl px-6 pt-12 pb-16">
		<section class="mb-8 flex flex-col gap-6 md:flex-row md:items-end md:justify-between">
			<div>
				<p class="muted-label mb-3">Cached articles from the extraction service</p>
				<h1 class="mb-2 font-serif text-4xl font-semibold text-ink">Library</h1>
				<p class="font-serif text-stone">
					{#if library}
						Browse previously extracted articles. <span class="font-mono text-sm"
							>{formatNumber(library.total)} articles cached</span>
					{:else}
						Browse previously extracted articles once the API is available.
					{/if}
				</p>
			</div>

			<div class="flex flex-wrap items-center gap-2">
				<span class="muted-label">Sort by</span>
				{#each SORT_OPTIONS as option (option.label)}
					<a
						class="btn-outline px-3 py-1.5 font-mono text-sm"
						data-active={query.sort === option.value}
						href={resolve(buildLink({ sort: option.value, page: 1 }))}>
						{option.label}
					</a>
				{/each}
			</div>
		</section>

		<form action="/library" class="mb-8 grid gap-4 lg:grid-cols-[minmax(0,1fr)_200px_180px_180px_auto]">
			<div class="relative">
				<input
					class="w-full border-2 border-mist bg-white px-4 py-3 pl-10 font-serif focus:border-ink focus:ring-0"
					name="q"
					placeholder="Search by title or domain..."
					type="text"
					value={query.q} />
				<i class="i-tabler-search pointer-events-none absolute top-1/2 left-3 h-5 w-5 -translate-y-1/2 text-fog"> </i>
			</div>

			<select class="border border-mist bg-white px-4 py-3 font-mono text-sm" name="domain">
				<option value="">All domains</option>
				{#each domainOptions as option (option)}
					<option selected={query.domain === option} value={option}>{option}</option>
				{/each}
			</select>

			<input
				class="border border-mist bg-white px-4 py-3 font-mono text-sm"
				name="date_from"
				type="date"
				value={query.date_from} />
			<input
				class="border border-mist bg-white px-4 py-3 font-mono text-sm"
				name="date_to"
				type="date"
				value={query.date_to} />

			<div class="flex gap-2">
				<input name="sort" type="hidden" value={query.sort} />
				<input name="per_page" type="hidden" value={query.per_page} />
				<button class="btn-ink px-5 py-3 text-sm font-semibold tracking-[0.18em] uppercase" type="submit">
					Filter
				</button>
			</div>
		</form>

		{#if library}
			<section class="stat-strip mb-8 grid gap-4 p-5 md:grid-cols-4">
				<div>
					<p class="text-3xl font-bold text-ink">{formatNumber(library.stats.total_articles)}</p>
					<p class="muted-label mt-1">Total Articles</p>
				</div>
				<div>
					<p class="text-3xl font-bold text-ink">{formatNumber(library.stats.total_reads)}</p>
					<p class="muted-label mt-1">Total Reads</p>
				</div>
				<div>
					<p class="text-3xl font-bold text-ink">{formatNumber(library.stats.unique_domains)}</p>
					<p class="muted-label mt-1">Unique Domains</p>
				</div>
				<div>
					<p class="text-3xl font-bold text-ink">
						{formatHoursFromMinutes(library.stats.total_reading_time_minutes)}
					</p>
					<p class="muted-label mt-1">Reading Time</p>
				</div>
			</section>
		{/if}

		<div class="grid gap-10 lg:grid-cols-[minmax(0,1fr)_280px]">
			<section>
				{#if hasResults}
					<div class="grid gap-6 md:grid-cols-2 xl:grid-cols-3">
						{#each library?.items ?? [] as item (item.id)}
							<a class="editorial-card block p-6" href={resolve(`/r/${item.id}`)}>
								<div class="mb-4 flex items-center gap-3">
									<div class="flex h-10 w-10 shrink-0 items-center justify-center border border-mist bg-paper">
										{#if item.favicon}
											<img alt="" class="h-6 w-6" src={item.favicon} />
										{:else}
											<span class="font-mono text-xs text-fog">{item.domain.slice(0, 2).toUpperCase()}</span>
										{/if}
									</div>
									<div class="min-w-0">
										<p class="truncate font-mono text-xs text-stone">{item.domain}</p>
										<p class="font-mono text-xs text-fog">{formatDate(item.fetched_at)}</p>
									</div>
								</div>

								<h2 class="mb-3 line-clamp-2 font-serif text-xl font-semibold text-ink">
									{item.title || item.url}
								</h2>
								<p class="mb-4 line-clamp-3 font-serif text-sm text-stone">
									{item.excerpt || 'No excerpt was stored for this cached article.'}
								</p>

								<div
									class="flex flex-wrap items-center justify-between gap-3 border-t border-mist pt-4 font-mono text-xs text-fog">
									<span>{formatWordCount(item.word_count)}</span>
									<span>{formatReadingTime(item.reading_time_minutes)}</span>
									<span>{formatNumber(item.hit_count)} reads</span>
								</div>
							</a>
						{/each}
					</div>

					<div class="mt-12 flex flex-wrap items-center justify-center gap-2">
						<a
							aria-disabled={query.page <= 1}
							class="btn-outline px-3 py-2 font-mono text-sm aria-disabled:pointer-events-none aria-disabled:opacity-50"
							href={resolve(buildLink({ page: Math.max(1, query.page - 1) }))}>
							← Prev
						</a>

						{#each Array.from({ length: totalPages }, (_, index) => index + 1).slice(Math.max(0, query.page - 3), Math.min(totalPages, query.page + 2)) as pageNumber (pageNumber)}
							<a
								class="btn-outline px-3 py-2 font-mono text-sm"
								data-active={pageNumber === query.page}
								href={resolve(buildLink({ page: pageNumber }))}>
								{pageNumber}
							</a>
						{/each}

						<a
							aria-disabled={query.page >= totalPages}
							class="btn-outline px-3 py-2 font-mono text-sm aria-disabled:pointer-events-none aria-disabled:opacity-50"
							href={resolve(buildLink({ page: Math.min(totalPages, query.page + 1) }))}>
							Next →
						</a>
					</div>
				{:else}
					<div class="editorial-panel p-10 text-center">
						<p class="muted-label mb-3">No matches</p>
						<p class="font-serif text-lg text-stone">
							Adjust the filters or extract a fresh article to seed the archive.
						</p>
					</div>
				{/if}
			</section>

			<aside class="space-y-6">
				<div class="editorial-panel p-6">
					<p class="muted-label mb-4">Top Domains</p>
					<div class="space-y-3">
						{#if topDomains.length}
							{#each topDomains as entry (entry.domain)}
								<a
									class="flex items-center justify-between border-b border-mist pb-3 last:border-b-0 last:pb-0"
									href={resolve(buildLink({ domain: entry.domain, page: 1 }))}>
									<span class="font-serif text-sm text-ink">{entry.domain}</span>
									<span class="font-mono text-xs text-fog">{formatNumber(entry.count)}</span>
								</a>
							{/each}
						{:else}
							<p class="font-serif text-sm text-stone">No domain stats yet.</p>
						{/if}
					</div>
				</div>

				<div class="editorial-panel p-6">
					<p class="muted-label mb-4">Current View</p>
					<div class="space-y-3 text-sm text-stone">
						<p class="font-serif">
							Page {query.page} of {totalPages}, sorted by
							<span class="font-mono text-ink">{query.sort}</span>.
						</p>
						<p class="font-serif">
							Showing {formatNumber(library?.items.length ?? 0)} items with a page size of {formatNumber(
								query.per_page
							)}.
						</p>
					</div>
				</div>
			</aside>
		</div>

		{#if error}
			<div class="mt-8 rounded-2xl border border-red-200 bg-white px-5 py-4 shadow-sm">
				<p class="muted-label mb-2 text-red-700">Library Error</p>
				<p class="font-serif text-sm text-graphite">{error}</p>
			</div>
		{/if}
	</div>

	<SiteFooter />
</div>
