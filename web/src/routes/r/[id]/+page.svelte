<script lang="ts">
	import { page } from '$app/state';
	import { extractArticleByUrl, getApiErrorMessage, getLibraryArticle } from '$lib/api';
	import ReaderPageView from '$lib/components/pages/ReaderPageView.svelte';
	import type { ExtractFormat, ExtractResponse } from '$lib/types';
	import { fileExtensionForFormat, mimeTypeForFormat, sanitizeExtractedHtml } from '$lib/utils';

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
</script>

<ReaderPageView
	{actionMessage}
	{activeView}
	{article}
	{errorMessage}
	{loadingFormat}
	onCopy={copyCurrent}
	onDownload={downloadCurrent}
	onSelectView={selectView}
	{renderedHtml}
	{variantContent} />
