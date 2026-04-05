<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { extractArticle, getApiErrorMessage, getLibrary, getLimits } from '$lib/api';
	import SiteHeader from '$lib/components/SiteHeader.svelte';
	import type { ExtractFormat, LibraryResponse, LimitsResponse, RateLimitHeaders } from '$lib/types';
	import { formatDate, formatNumber, formatReadingTime, formatWordCount } from '$lib/utils';

	let url = $state('');
	let format = $state<ExtractFormat>('markdown');
	let includeFrontmatter = $state(true);
	let includeReferences = $state(false);
	let stripImages = $state(false);
	let loading = $state(false);
	let errorMessage = $state<string | null>(null);
	let recent = $state<LibraryResponse | null>(null);
	let recentError = $state<string | null>(null);
	let limits = $state<LimitsResponse | null>(null);
	let rateLimit = $state<RateLimitHeaders | null>(null);

	$effect(() => {
		let cancelled = false;

		void (async () => {
			const [recentResult, limitsResult] = await Promise.allSettled([
				getLibrary(fetch, { per_page: 3, sort: 'recent' }),
				getLimits(fetch)
			]);

			if (cancelled) return;

			if (recentResult.status === 'fulfilled') {
				recent = recentResult.value.data;
				recentError = null;
			} else {
				recent = null;
				recentError = getApiErrorMessage(recentResult.reason);
			}

			if (limitsResult.status === 'fulfilled') {
				limits = limitsResult.value.data;
				rateLimit = { limit: limits.requests_limit, remaining: limits.requests_remaining };
			}
		})();

		return () => {
			cancelled = true;
		};
	});

	async function submitExtraction() {
		errorMessage = null;

		try {
			new URL(url);
		} catch {
			errorMessage = 'Enter a valid URL, including the protocol.';
			return;
		}

		loading = true;

		try {
			const result = await extractArticle(fetch, {
				url,
				format,
				include_frontmatter: includeFrontmatter,
				include_references: includeReferences,
				strip_images: stripImages
			});

			rateLimit = result.rateLimit;

			if (!result.data.id) {
				errorMessage = 'The article was extracted, but no cache id was returned for the reader route.';
				return;
			}

			await goto(resolve(`/r/${result.data.id}`));
		} catch (error) {
			errorMessage = getApiErrorMessage(error);
		} finally {
			loading = false;
		}
	}
</script>

<svelte:head>
	<title>Lectito</title>
	<meta name="description" content="Extract clean, readable articles from any URL and browse cached reads in Lectito." />
</svelte:head>

<SiteHeader active="home" />

<main class="mx-auto max-w-6xl px-6 py-16">
	<section class="mx-auto max-w-3xl">
		<div class="mb-12 text-center">
			<p class="muted-label mb-4">Fast Rust extraction for research, archives, and reading queues</p>
			<h1 class="mb-4 font-serif text-4xl font-semibold tracking-tight text-ink md:text-5xl">Paste a URL to extract</h1>
			<p class="mx-auto max-w-2xl font-serif text-lg text-stone">
				Clean, readable articles from any web page. No clutter, no tracking detours, just the content you came for.
			</p>
		</div>

		<div class="editorial-panel p-8 md:p-10">
			<form
				class="space-y-6"
				onsubmit={(event) => {
					event.preventDefault();
					void submitExtraction();
				}}>
				<div>
					<label class="muted-label mb-2 block" for="article-url">Article URL</label>
					<input
						id="article-url"
						bind:value={url}
						class="w-full border-2 border-mist bg-[rgba(250,250,250,0.88)] px-4 py-4 font-serif text-lg text-ink focus:border-ink focus:ring-0"
						placeholder="https://example.com/article"
						type="url"
						required />
				</div>

				<div class="grid gap-4 md:grid-cols-3">
					<div>
						<label class="muted-label mb-2 block" for="format">Format</label>
						<select
							id="format"
							bind:value={format}
							class="w-full border border-mist bg-white px-3 py-3 font-mono text-sm">
							<option value="markdown">Markdown</option>
							<option value="html">HTML</option>
							<option value="text">Plain Text</option>
							<option value="json">JSON</option>
						</select>
					</div>
					<div>
						<p class="muted-label mb-2">Options</p>
						<div class="space-y-2 py-1 text-sm">
							<label class="flex cursor-pointer items-center gap-3 font-serif text-graphite">
								<input bind:checked={includeFrontmatter} class="text-ink focus:ring-0" type="checkbox" />
								<span>Include frontmatter</span>
							</label>
							<label class="flex cursor-pointer items-center gap-3 font-serif text-graphite">
								<input bind:checked={stripImages} class="text-ink focus:ring-0" type="checkbox" />
								<span>Strip images</span>
							</label>
						</div>
					</div>
					<div>
						<p class="muted-label mb-2">Output</p>
						<div class="space-y-2 py-1 text-sm">
							<label class="flex cursor-pointer items-center gap-3 font-serif text-graphite">
								<input bind:checked={includeReferences} class="text-ink focus:ring-0" type="checkbox" />
								<span>Link references</span>
							</label>
							<div
								class="rounded-xl border border-dashed border-mist bg-[rgba(255,255,255,0.72)] px-3 py-3 text-xs text-stone">
								Reader view opens after extraction and keeps the cached article on hand for later.
							</div>
						</div>
					</div>
				</div>

				<button
					class="btn-ink w-full px-5 py-4 text-sm font-semibold tracking-[0.24em] uppercase disabled:cursor-not-allowed disabled:opacity-70"
					disabled={loading}
					type="submit">
					{#if loading}
						<span class="inline-flex items-center gap-3">
							<span class="h-4 w-4 animate-spin rounded-full border-2 border-white/20 border-t-white"></span>
							<span>Extracting</span>
						</span>
					{:else}
						Extract Content
					{/if}
				</button>
			</form>
		</div>

		<div class="mt-5 flex flex-wrap items-center justify-between gap-4 text-sm">
			<div class="font-serif text-stone">
				{#if rateLimit?.remaining !== undefined}
					<span>{formatNumber(rateLimit.remaining)} requests left in the current window.</span>
				{:else}
					<span>Free public API with request limits surfaced on every response.</span>
				{/if}
			</div>
			<a
				class="border-b border-stone font-medium text-stone hover:border-ink hover:text-ink"
				href={resolve('/about#rate-limits')}>
				View rate limits
			</a>
		</div>
	</section>

	<section class="mt-16 grid gap-8 md:grid-cols-3">
		<div class="text-center">
			<div class="mx-auto mb-4 flex h-12 w-12 items-center justify-center border-2 border-ink">
				<span class="flex items-center">
					<i class="i-tabler-bolt ml-1 h-6 w-6 text-yellow-500"></i>
				</span>
			</div>
			<h2 class="mb-2 font-semibold text-ink">Fast</h2>
			<p class="font-serif text-sm text-stone">
				Server-side caching and Rust extraction keep repeat reads responsive without sacrificing detail.
			</p>
		</div>
		<div class="text-center">
			<div class="mx-auto mb-4 flex h-12 w-12 items-center justify-center border-2 border-ink">
				<span class="flex items-center">
					<i class="i-tabler-book ml-1 h-6 w-6 text-blue-500"></i>
				</span>
			</div>
			<h2 class="mb-2 font-semibold text-ink">Clean</h2>
			<p class="font-serif text-sm text-stone">
				Focused reader layouts, metadata capture, and multiple export formats make the result immediately usable.
			</p>
		</div>
		<div class="text-center">
			<div class="mx-auto mb-4 flex h-12 w-12 items-center justify-center border-2 border-ink">
				<span class="flex items-center">
					<i class="i-tabler-api ml-1 h-6 w-6 text-green-500"></i>
				</span>
			</div>
			<h2 class="mb-2 font-semibold text-ink">Public API</h2>
			<p class="font-serif text-sm text-stone">
				POST and GET extraction endpoints, a cached library, and rate-limit headers for predictable integrations.
			</p>
		</div>
	</section>

	<section class="mt-20 border-t border-mist pt-12">
		<div class="mb-8 flex items-center justify-between gap-4">
			<div>
				<p class="muted-label mb-2">Archive Preview</p>
				<h2 class="font-serif text-2xl font-semibold text-ink">Recently Extracted</h2>
			</div>
			<a
				class="border-b border-stone text-sm font-medium text-stone hover:border-ink hover:text-ink"
				href={resolve('/library')}>
				View Library →
			</a>
		</div>

		{#if recent?.items?.length}
			<div class="space-y-4">
				{#each recent.items as item (item.id)}
					<a class="editorial-card block p-5" href={resolve(`/r/${item.id}`)}>
						<div class="flex gap-4">
							<div class="flex h-12 w-12 shrink-0 items-center justify-center border border-mist bg-paper">
								{#if item.favicon}
									<img alt="" class="h-6 w-6" src={item.favicon} />
								{:else}
									<span class="font-mono text-xs text-fog">{item.domain.slice(0, 2).toUpperCase()}</span>
								{/if}
							</div>
							<div class="min-w-0 flex-1">
								<h3 class="mb-1 line-clamp-2 font-serif text-lg font-semibold text-ink">
									{item.title || item.url}
								</h3>
								<p class="line-clamp-2 font-serif text-sm text-stone">
									{item.excerpt || 'No excerpt was captured for this cached article yet.'}
								</p>
								<div class="mt-3 flex flex-wrap items-center gap-4 font-mono text-xs text-fog">
									<span>{item.domain}</span>
									<span>{formatWordCount(item.word_count)}</span>
									<span>{formatReadingTime(item.reading_time_minutes)}</span>
									<span>{formatDate(item.fetched_at)}</span>
								</div>
							</div>
						</div>
					</a>
				{/each}
			</div>
		{:else}
			<div class="editorial-panel p-8 text-center">
				<p class="font-serif text-stone">
					{recentError || 'No cached articles yet. Extract one above to seed the library.'}
				</p>
			</div>
		{/if}
	</section>
</main>

{#if errorMessage}
	<div class="fixed inset-x-0 bottom-6 z-50 mx-auto max-w-xl px-6">
		<div class="rounded-2xl border border-red-200 bg-white px-5 py-4 shadow-xl">
			<p class="muted-label mb-2 text-red-700">Extraction Error</p>
			<p class="font-serif text-sm text-graphite">{errorMessage}</p>
		</div>
	</div>
{/if}
