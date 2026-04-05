<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { extractArticle, getApiErrorMessage, getLibrary, getLimits } from '$lib/api';
	import HomePageView from '$lib/components/pages/HomePageView.svelte';
	import type { ExtractFormat, LibraryResponse, LimitsResponse, RateLimitHeaders } from '$lib/types';

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

<HomePageView
	bind:format
	bind:includeFrontmatter
	bind:includeReferences
	bind:stripImages
	bind:url
	{errorMessage}
	{loading}
	onSubmit={submitExtraction}
	{rateLimit}
	{recent}
	{recentError} />
