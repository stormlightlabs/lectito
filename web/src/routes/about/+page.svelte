<script lang="ts">
  import { resolve } from '$app/paths';
  import { ABOUT } from '$lib/content';
  import { LINKS } from '$lib/content/docs';
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
            <a class="block font-serif text-stone hover:text-ink" href="#api-reference">Docs Overview</a>
            <a class="block font-serif text-stone hover:text-ink" href={resolve('/docs')}>Open API Docs</a>
          </div>
        </div>
        <div>
          <p class="muted-label mb-3">Resources</p>
          <div class="space-y-2 text-sm">
            <a class="block font-serif text-stone hover:text-ink" href={resolve('/docs')}>API Docs</a>
            <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
            <a class="block font-serif text-stone hover:text-ink" href={LINKS.swagger.href}>{LINKS.swagger.label}</a>
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
          <div class="editorial-panel max-w-3xl p-6">
            <p class="font-serif text-stone">
              The detailed API reference now lives on a dedicated docs page that reads the live OpenAPI document, renders
              endpoint details, and keeps request examples in sync with the published schema.
            </p>
            <div class="mt-4 flex flex-wrap items-center gap-4 text-sm">
              <a class="btn-ink px-4 py-2 font-semibold tracking-[0.16em] uppercase" href={resolve('/docs')}>
                Open API Docs
              </a>
              <a
                class="border-b border-stone font-medium text-stone hover:border-ink hover:text-ink"
                // eslint-disable-next-line svelte/no-navigation-without-resolve
                href={LINKS.swagger.href}>
                {LINKS.swagger.label}
              </a>
              <a
                class="border-b border-stone font-medium text-stone hover:border-ink hover:text-ink"
                // eslint-disable-next-line svelte/no-navigation-without-resolve
                href={LINKS.openApiJson.href}>
                {LINKS.openApiJson.label}
              </a>
            </div>
          </div>
        </div>
      </section>
    </main>
  </div>
</div>
