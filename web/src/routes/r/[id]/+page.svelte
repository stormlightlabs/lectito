<script lang="ts">
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { extractArticleByUrl, getApiErrorMessage, getLibraryArticle } from '$lib/api';
	import { READER } from '$lib/content';
	import type { ExtractFormat, ExtractResponse } from '$lib/types';
	import {
		fileExtensionForFormat,
		formatDate,
		formatDateTime,
		formatReadingTime,
		formatWordCount,
		getInitials,
		mimeTypeForFormat,
		sanitizeExtractedHtml
	} from '$lib/utils';

	type ViewMode = 'rendered' | ExtractFormat;

	let article = $state<ExtractResponse | null>(null);
	let activeView = $state<ViewMode>('html');
	let variantContent = $state<Partial<Record<ExtractFormat, string>>>({});
	let renderedHtml = $state('');
	let loadingFormat = $state<ExtractFormat | null>(null);
	let actionMessage = $state<string | null>(null);
	let errorMessage = $state<string | null>(null);

	$effect(() => {
		const id = page.params.id;
		let cancelled = false;

		void (async () => {
			if (!id) {
				article = null;
				variantContent = {};
				renderedHtml = '';
				errorMessage = 'No cached article id was provided.';
				return;
			}

			try {
				const articleResult = await getLibraryArticle(fetch, id);
				if (cancelled) return;

				article = articleResult.data;
				variantContent = { [article.format]: article.content };
				activeView = article.format === 'html' ? 'rendered' : article.format;
				renderedHtml = article.format === 'html' ? sanitizeExtractedHtml(article.content) : '';
				errorMessage = null;

				if (article.format !== 'html') {
					try {
						const rendered = await extractArticleByUrl(fetch, { url: article.url, format: 'html' });

						if (cancelled) return;

						variantContent = { ...variantContent, html: rendered.data.content };
						renderedHtml = sanitizeExtractedHtml(rendered.data.content);
					} catch {
						// Keep the primary format even if the rendered variant is unavailable.
					}
				}
			} catch (requestError) {
				if (cancelled) return;
				article = null;
				variantContent = {};
				renderedHtml = '';
				errorMessage = getApiErrorMessage(requestError);
			}
		})();

		return () => {
			cancelled = true;
		};
	});

	async function ensureFormat(format: ExtractFormat) {
		if (!article || variantContent[format]) return true;

		loadingFormat = format;
		errorMessage = null;

		try {
			const result = await extractArticleByUrl(fetch, { url: article.url, format });
			variantContent = { ...variantContent, [format]: result.data.content };

			if (format === 'html') {
				renderedHtml = sanitizeExtractedHtml(result.data.content);
			}

			return true;
		} catch (error) {
			errorMessage = getApiErrorMessage(error);
			return false;
		} finally {
			loadingFormat = null;
		}
	}

	async function selectView(view: ViewMode) {
		if (view === 'rendered') {
			const ready = await ensureFormat('html');
			if (ready) activeView = view;
			return;
		}

		const ready = await ensureFormat(view);
		if (ready) activeView = view;
	}

	function currentFormat(): ExtractFormat {
		return activeView === 'rendered' ? 'html' : activeView;
	}

	function currentContent() {
		return variantContent[currentFormat()] ?? '';
	}

	async function copyCurrent() {
		try {
			await navigator.clipboard.writeText(currentContent());
			actionMessage = 'Copied to clipboard.';
		} catch {
			actionMessage = 'Clipboard access failed.';
		}

		setTimeout(() => {
			actionMessage = null;
		}, 1600);
	}

	function downloadCurrent() {
		const format = currentFormat();
		const blob = new Blob([currentContent()], { type: mimeTypeForFormat(format) });
		const href = URL.createObjectURL(blob);
		const link = document.createElement('a');
		link.href = href;
		link.download = `lectito-${article?.id ?? 'article'}.${fileExtensionForFormat(format)}`;
		link.click();
		URL.revokeObjectURL(href);
	}

	const VIEW_OPTIONS: { value: ViewMode; label: string }[] = [
		{ value: 'rendered', label: 'Rendered' },
		{ value: 'markdown', label: 'Markdown' },
		{ value: 'html', label: 'HTML' },
		{ value: 'text', label: 'Text' },
		{ value: 'json', label: 'JSON' }
	];
</script>

{#if article}
	<div class="mx-auto max-w-3xl px-6 pt-12 pb-16">
		<div class="mb-6 flex flex-wrap items-center gap-3 text-sm">
			{#if article.metadata.favicon}
				<img alt="" class="h-5 w-5" src={article.metadata.favicon} />
			{/if}
			<span class="font-mono text-stone">
				{article.metadata.site_name || new URL(article.url).hostname}
			</span>
			<span class="text-mist">•</span>
			<span class="font-serif text-stone">{formatDate(article.metadata.date || article.extracted_at)}</span>
		</div>

		<h1 class="mb-6 font-serif text-4xl leading-tight font-bold text-ink md:text-5xl">
			{article.metadata.title || article.url}
		</h1>

		<div class="mb-8 flex flex-wrap items-center gap-6 border-b border-mist pb-8">
			<div class="flex items-center gap-3">
				<div class="flex h-11 w-11 items-center justify-center rounded-full bg-ink text-sm font-semibold text-white">
					{getInitials(article.metadata.author)}
				</div>
				<div>
					<p class="font-semibold text-ink">
						{article.metadata.author || READER.authorFallback}
					</p>
					<p class="text-xs text-stone">{article.metadata.site_name || article.url}</p>
				</div>
			</div>

			<div class="ml-auto flex flex-wrap items-center gap-4 font-mono text-sm text-stone">
				<span>{formatWordCount(article.metadata.word_count)}</span>
				<span>•</span>
				<span>{formatReadingTime(article.metadata.reading_time_minutes)}</span>
			</div>
		</div>

		<div class="mb-8 flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
			<div class="flex flex-wrap items-center gap-2">
				<span class="muted-label mr-2">View as</span>
				{#each VIEW_OPTIONS as option (option.label)}
					<button
						class="btn-outline px-3 py-1.5 font-mono text-sm"
						data-active={activeView === option.value}
						disabled={loadingFormat !== null}
						onclick={() => void selectView(option.value)}
						type="button">
						{option.label}
					</button>
				{/each}
			</div>

			<div class="flex flex-wrap items-center gap-2">
				<button class="btn-outline px-4 py-2 text-sm font-medium" onclick={() => void copyCurrent()} type="button">
					Copy
				</button>
				<button class="btn-ink px-4 py-2 text-sm font-medium" onclick={() => void downloadCurrent()} type="button">
					Download
				</button>
			</div>
		</div>

		{#if loadingFormat}
			<div class="mb-6 rounded-2xl border border-mist bg-white px-4 py-3 text-sm text-stone">
				Loading {loadingFormat} view…
			</div>
		{/if}

		{#if errorMessage}
			<div class="mb-6 rounded-2xl border border-red-200 bg-white px-4 py-3 text-sm text-graphite">
				{errorMessage}
			</div>
		{/if}

		{#if activeView === 'rendered'}
			<article class="article-body">
				<!-- eslint-disable-next-line svelte/no-at-html-tags -->
				{@html renderedHtml}
			</article>
		{:else}
			<pre class="raw-view">{variantContent[activeView] ?? ''}</pre>
		{/if}

		<div class="mt-16 flex flex-col gap-4 border-t border-mist pt-8 md:flex-row md:items-center md:justify-between">
			<div class="flex flex-wrap items-center gap-4">
				<a
					class="border-b border-stone text-sm font-medium text-stone hover:border-ink hover:text-ink"
					href={resolve('/')}>
					{READER.nav.extractAnother}
				</a>
				<a
					class="border-b border-stone text-sm font-medium text-stone hover:border-ink hover:text-ink"
					href={resolve('/library')}>
					{READER.nav.browseLibrary}
				</a>
			</div>

			<div class="flex flex-wrap items-center gap-2 font-mono text-xs text-fog">
				<span>Extracted:</span>
				<span>{formatDateTime(article.extracted_at)}</span>
				<span>•</span>
				<span class:text-accent={article.cached}>Cached</span>
			</div>
		</div>

		{#if actionMessage}
			<p class="mt-4 font-mono text-xs text-stone">{actionMessage}</p>
		{/if}
	</div>
{:else}
	<div class="mx-auto max-w-3xl px-6 py-20">
		<div class="editorial-panel p-10 text-center">
			<p class="muted-label mb-3">{READER.unavailable.label}</p>
			<p class="font-serif text-lg text-stone">
				{errorMessage || READER.unavailable.body}
			</p>
			<div class="mt-6">
				<a
					class="btn-ink inline-flex px-5 py-3 text-sm font-semibold tracking-[0.18em] uppercase"
					href={resolve('/library')}>
					{READER.unavailable.back}
				</a>
			</div>
		</div>
	</div>
{/if}
