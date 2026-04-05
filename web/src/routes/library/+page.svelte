<script lang="ts">
	import { page } from '$app/state';
	import { getApiErrorMessage, getLibrary } from '$lib/api';
	import LibraryPageView from '$lib/components/pages/LibraryPageView.svelte';
	import type { LibraryResponse, LibrarySort } from '$lib/types';
	import { SvelteURLSearchParams } from 'svelte/reactivity';

	function parsePositiveInt(value: string | null, fallback: number) {
		const parsed = Number(value);
		return Number.isInteger(parsed) && parsed > 0 ? parsed : fallback;
	}

	function parseSort(value: string | null): LibrarySort {
		return value === 'popular' || value === 'alpha' ? value : 'recent';
	}

	const currentQuery = $derived.by(() => ({
		page: parsePositiveInt(page.url.searchParams.get('page'), 1),
		per_page: Math.min(parsePositiveInt(page.url.searchParams.get('per_page'), 12), 100),
		sort: parseSort(page.url.searchParams.get('sort')),
		q: page.url.searchParams.get('q') ?? '',
		domain: page.url.searchParams.get('domain') ?? '',
		date_from: page.url.searchParams.get('date_from') ?? '',
		date_to: page.url.searchParams.get('date_to') ?? ''
	}));

	let library = $state<LibraryResponse | null>(null);
	let error = $state<string | null>(null);

	function withQuery(overrides: Record<string, string | number | null | undefined>) {
		const params = new SvelteURLSearchParams();
		const merged = {
			page: currentQuery.page,
			per_page: currentQuery.per_page,
			sort: currentQuery.sort,
			q: currentQuery.q,
			domain: currentQuery.domain,
			date_from: currentQuery.date_from,
			date_to: currentQuery.date_to,
			...overrides
		};

		for (const [key, value] of Object.entries(merged)) {
			if (value === undefined || value === null || value === '') continue;
			params.set(key, String(value));
		}

		const search = params.toString();
		return search ? `/library?${search}` : '/library';
	}

	$effect(() => {
		const query = currentQuery;
		let cancelled = false;

		void (async () => {
			try {
				const result = await getLibrary(fetch, query);
				if (cancelled) return;
				library = result.data;
				error = null;
			} catch (requestError) {
				if (cancelled) return;
				library = null;
				error = getApiErrorMessage(requestError);
			}
		})();

		return () => {
			cancelled = true;
		};
	});
</script>

<LibraryPageView buildLink={withQuery} {error} {library} query={currentQuery} />
