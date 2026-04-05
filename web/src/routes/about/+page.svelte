<script lang="ts">
	import { resolve } from '$app/paths';
	import { ABOUT } from '$lib/content';
</script>

<svelte:head>
	<title>{ABOUT.meta.title}</title>
	<meta name="description" content={ABOUT.meta.description} />
</svelte:head>

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
				<h1 class="mb-6 font-serif text-4xl font-semibold text-ink">{ABOUT.about.heading}</h1>
				<div class="max-w-none space-y-5 font-serif text-lg leading-relaxed text-charcoal">
					<p>{ABOUT.about.paragraphs[0]}</p>
					<p class="text-stone">{ABOUT.about.paragraphs[1]}</p>
					<p class="text-stone">{ABOUT.about.paragraphs[2]}</p>
				</div>
			</section>

			<section id="features">
				<h2 class="mb-6 font-serif text-3xl font-semibold text-ink">{ABOUT.features.heading}</h2>
				<div class="grid gap-6 md:grid-cols-2">
					{#each ABOUT.features.items as item (item.heading)}
						<div class="editorial-card p-6">
							<h3 class="mb-2 font-semibold text-ink">{item.heading}</h3>
							<p class="font-serif text-sm text-stone">{item.body}</p>
						</div>
					{/each}
				</div>
			</section>

			<section id="rate-limits">
				<h2 class="mb-6 font-serif text-3xl font-semibold text-ink">{ABOUT.rateLimits.heading}</h2>
				<p class="mb-6 font-serif text-stone">
					{ABOUT.rateLimits.intro}
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
					<h2 class="mb-6 font-serif text-3xl font-semibold text-ink">{ABOUT.apiReference.heading}</h2>
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
								>{String.raw`curl -X POST http://localhost:3000/api/v1/extract \
  -H "Content-Type: application/json" \
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
