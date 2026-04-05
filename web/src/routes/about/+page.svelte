<script lang="ts">
	import { resolve } from '$app/paths';
	import SiteHeader from '$lib/components/SiteHeader.svelte';
</script>

<svelte:head>
	<title>About · Lectito</title>
	<meta name="description" content="About Lectito, its API surface, rate limits, and example extraction workflows." />
</svelte:head>

<SiteHeader active="about" />

<div class="mx-auto max-w-6xl px-6 py-12">
	<div class="grid gap-12 lg:grid-cols-[240px_minmax(0,1fr)]">
		<aside class="hidden lg:block">
			<nav class="sticky top-8 space-y-6">
				<div>
					<p class="muted-label mb-3">Overview</p>
					<div class="space-y-2 text-sm">
						<a class="block font-serif text-stone hover:text-ink" href="#about">About Lectito</a>
						<a class="block font-serif text-stone hover:text-ink" href="#features">Features</a>
						<a class="block font-serif text-stone hover:text-ink" href="#rate-limits">Rate Limits</a>
					</div>
				</div>
				<div>
					<p class="muted-label mb-3">API Reference</p>
					<div class="space-y-2 text-sm">
						<a class="block font-serif text-stone hover:text-ink" href="#api-reference">POST /extract</a>
						<a class="block font-serif text-stone hover:text-ink" href="#get-extract">GET /extract</a>
						<a class="block font-serif text-stone hover:text-ink" href="#library">GET /library</a>
						<a class="block font-serif text-stone hover:text-ink" href="#health">GET /health</a>
						<a class="block font-serif text-stone hover:text-ink" href="#limits">GET /limits</a>
					</div>
				</div>
				<div>
					<p class="muted-label mb-3">Resources</p>
					<div class="space-y-2 text-sm">
						<a class="block font-serif text-stone hover:text-ink" href={resolve('/library')}>Library</a>
						<a class="block font-serif text-stone hover:text-ink" href={resolve('/')}>Extract</a>
					</div>
				</div>
			</nav>
		</aside>

		<main class="space-y-16">
			<section id="about">
				<h1 class="mb-6 font-serif text-4xl font-semibold text-ink">About Lectito</h1>
				<div class="max-w-none space-y-5 font-serif text-lg leading-relaxed text-charcoal">
					<p>
						Lectito is a free, open-source service for extracting readable content from web pages. It removes clutter,
						isolates the main article, and returns clean output as Markdown, HTML, plain text, or JSON.
					</p>
					<p class="text-stone">
						The stack pairs a Rust extraction engine with an Axum API and a Svelte client. Cached results back the
						library view, keeping frequent reads fast and making earlier extractions easy to revisit.
					</p>
					<p class="text-stone">
						The current public API is designed for research workflows, reading queues, and lightweight automation where
						consistent output matters more than scraping raw page chrome.
					</p>
				</div>
			</section>

			<section id="features">
				<h2 class="mb-6 font-serif text-3xl font-semibold text-ink">Features</h2>
				<div class="grid gap-6 md:grid-cols-2">
					<div class="editorial-card p-6">
						<h3 class="mb-2 font-semibold text-ink">Smart Content Extraction</h3>
						<p class="font-serif text-sm text-stone">
							Removes ads, sidebars, navigation, and other low-value blocks so the article body is easier to read or
							reprocess.
						</p>
					</div>
					<div class="editorial-card p-6">
						<h3 class="mb-2 font-semibold text-ink">Multiple Output Formats</h3>
						<p class="font-serif text-sm text-stone">
							Request HTML, Markdown, text, or JSON depending on whether you need reader mode, export, or structured
							ingestion.
						</p>
					</div>
					<div class="editorial-card p-6">
						<h3 class="mb-2 font-semibold text-ink">Cached Library</h3>
						<p class="font-serif text-sm text-stone">
							Previously extracted articles are kept in Postgres, powering pagination, search, popularity sorting, and
							revisit flows.
						</p>
					</div>
					<div class="editorial-card p-6">
						<h3 class="mb-2 font-semibold text-ink">Operational Controls</h3>
						<p class="font-serif text-sm text-stone">
							Rate limiting, blocklists, and admin endpoints give the server enough leverage to stay public-facing
							without being reckless.
						</p>
					</div>
				</div>
			</section>

			<section id="rate-limits">
				<h2 class="mb-6 font-serif text-3xl font-semibold text-ink">Rate Limits</h2>
				<p class="mb-6 font-serif text-stone">
					The API enforces per-IP limits across three windows so the public service stays stable under mixed interactive
					and scripted traffic.
				</p>
				<div class="overflow-x-auto">
					<table class="w-full text-left text-sm">
						<thead>
							<tr class="border-b-2 border-ink">
								<th class="py-3 font-semibold">Window</th>
								<th class="py-3 font-semibold">Limit</th>
								<th class="py-3 font-semibold">Reset</th>
							</tr>
						</thead>
						<tbody class="font-mono">
							<tr class="border-b border-mist">
								<td class="py-3">Per Minute</td>
								<td class="py-3">60 requests</td>
								<td class="py-3 text-stone">Every minute</td>
							</tr>
							<tr class="border-b border-mist">
								<td class="py-3">Per Hour</td>
								<td class="py-3">600 requests</td>
								<td class="py-3 text-stone">Every hour</td>
							</tr>
							<tr>
								<td class="py-3">Per Day</td>
								<td class="py-3">5,000 requests</td>
								<td class="py-3 text-stone">Daily (UTC)</td>
							</tr>
						</tbody>
					</table>
				</div>
			</section>

			<section id="api-reference" class="space-y-12">
				<div>
					<h2 class="mb-6 font-serif text-3xl font-semibold text-ink">API Reference</h2>
				</div>

				<div class="border-l-[3px] border-ink pl-6">
					<div class="mb-4 flex items-center gap-3">
						<span class="bg-ink px-2 py-1 font-mono text-xs font-semibold text-white">POST</span>
						<code class="font-mono text-lg">/api/v1/extract</code>
					</div>
					<p class="mb-4 font-serif text-stone">
						Extract article content from a URL and return the requested output format.
					</p>
					<div class="raw-view">
						<code
							>{`{
  "url": "https://example.com/article",
  "format": "markdown",
  "include_frontmatter": true,
  "include_references": false,
  "strip_images": false,
  "content_selector": null
}`}</code>
					</div>
				</div>

				<div id="get-extract" class="border-l-[3px] border-ink pl-6">
					<div class="mb-4 flex items-center gap-3">
						<span class="bg-stone px-2 py-1 font-mono text-xs font-semibold text-white">GET</span>
						<code class="font-mono text-lg">/api/v1/extract</code>
					</div>
					<p class="mb-4 font-serif text-stone">
						GET variant for browser-friendly integrations and lightweight automation.
					</p>
					<div class="raw-view">
						<code>GET /api/v1/extract?url=https%3A%2F%2Fexample.com%2Farticle&amp;format=markdown</code>
					</div>
				</div>

				<div id="library" class="border-l-[3px] border-ink pl-6">
					<div class="mb-4 flex items-center gap-3">
						<span class="bg-stone px-2 py-1 font-mono text-xs font-semibold text-white">GET</span>
						<code class="font-mono text-lg">/api/v1/library</code>
					</div>
					<p class="mb-4 font-serif text-stone">
						Returns paginated cached articles with title, excerpt, source metadata, read counts, and aggregate library
						stats.
					</p>
					<ul class="space-y-2 font-mono text-sm text-charcoal">
						<li><code>page</code> - page number, default 1</li>
						<li><code>per_page</code> - page size, default 20, max 100</li>
						<li><code>sort</code> - recent, popular, or alpha</li>
						<li><code>q</code> - title or domain search</li>
						<li><code>domain</code>, <code>date_from</code>, <code>date_to</code> - optional filters</li>
					</ul>
				</div>

				<div id="health" class="border-l-[3px] border-ink pl-6">
					<div class="mb-4 flex items-center gap-3">
						<span class="bg-stone px-2 py-1 font-mono text-xs font-semibold text-white">GET</span>
						<code class="font-mono text-lg">/api/v1/health</code>
					</div>
					<div class="raw-view">
						<code
							>{`{
  "status": "ok",
  "version": "1.0.0",
  "database": "ok"
}`}</code>
					</div>
				</div>

				<div id="limits" class="border-l-[3px] border-ink pl-6">
					<div class="mb-4 flex items-center gap-3">
						<span class="bg-stone px-2 py-1 font-mono text-xs font-semibold text-white">GET</span>
						<code class="font-mono text-lg">/api/v1/limits</code>
					</div>
					<div class="raw-view">
						<code
							>{`{
  "requests_remaining": 55,
  "requests_limit": 60,
  "window_seconds": 60,
  "reset_at": "2026-04-05T12:01:00Z"
}`}</code>
					</div>
				</div>
			</section>

			<section id="example-usage">
				<h2 class="mb-6 font-serif text-3xl font-semibold text-ink">Example Usage</h2>
				<div class="space-y-6">
					<div>
						<p class="muted-label mb-3">cURL</p>
						<div class="raw-view">
							<code
								>{`curl -X POST http://localhost:3000/api/v1/extract \\
  -H "Content-Type: application/json" \\
  -d '{"url":"https://example.com/article","format":"markdown"}'`}</code>
						</div>
					</div>
					<div>
						<p class="muted-label mb-3">JavaScript</p>
						<div class="raw-view">
							<code
								>{`const response = await fetch('/api/v1/extract', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    url: 'https://example.com/article',
    format: 'markdown'
  })
});

const result = await response.json();
console.log(result.content);`}</code>
						</div>
					</div>
				</div>
			</section>

			<section id="error-codes">
				<h2 class="mb-6 font-serif text-3xl font-semibold text-ink">Error Codes</h2>
				<div class="overflow-x-auto">
					<table class="w-full text-left text-sm">
						<thead>
							<tr class="border-b-2 border-ink">
								<th class="py-3 font-semibold">Code</th>
								<th class="py-3 font-semibold">Meaning</th>
								<th class="py-3 font-semibold">Description</th>
							</tr>
						</thead>
						<tbody class="font-mono">
							<tr class="border-b border-mist">
								<td class="py-3">400</td>
								<td class="py-3 font-serif">Bad Request</td>
								<td class="py-3 text-stone">Invalid URL or malformed request body</td>
							</tr>
							<tr class="border-b border-mist">
								<td class="py-3">403</td>
								<td class="py-3 font-serif">Forbidden</td>
								<td class="py-3 text-stone">Blocked by spam controls or admin rules</td>
							</tr>
							<tr class="border-b border-mist">
								<td class="py-3">429</td>
								<td class="py-3 font-serif">Too Many Requests</td>
								<td class="py-3 text-stone">Rate limit exceeded, Retry-After header provided</td>
							</tr>
							<tr class="border-b border-mist">
								<td class="py-3">502</td>
								<td class="py-3 font-serif">Bad Gateway</td>
								<td class="py-3 text-stone">Upstream fetch failure</td>
							</tr>
							<tr>
								<td class="py-3">504</td>
								<td class="py-3 font-serif">Gateway Timeout</td>
								<td class="py-3 text-stone">Upstream fetch timed out before extraction completed</td>
							</tr>
						</tbody>
					</table>
				</div>
			</section>
		</main>
	</div>
</div>
