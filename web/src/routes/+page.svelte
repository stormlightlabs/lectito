<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { extractArticle, getApiErrorMessage, getLibrary, getLimits } from '$lib/api';
  import { HOME } from '$lib/content';
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
      errorMessage = HOME.errors.invalidUrl;
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
        errorMessage = HOME.errors.noCacheId;
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
  <title>{HOME.meta.title}</title>
  <meta name="description" content={HOME.meta.description} />
</svelte:head>

<main class="mx-auto max-w-6xl px-6 py-16">
  <section class="mx-auto max-w-3xl">
    <div class="mb-12 text-center">
      <p class="muted-label mb-4">{HOME.hero.label}</p>
      <h1 class="mb-4 font-serif text-4xl font-semibold tracking-tight text-ink md:text-5xl">{HOME.hero.heading}</h1>
      <p class="mx-auto max-w-2xl font-serif text-lg text-stone">
        {HOME.hero.body}
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
          <label class="muted-label mb-2 block" for="article-url">{HOME.form.urlLabel}</label>
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
                {HOME.form.readerHint}
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
              <span>{HOME.form.submitLoading}</span>
            </span>
          {:else}
            {HOME.form.submitIdle}
          {/if}
        </button>
      </form>
    </div>

    <div class="mt-5 flex flex-wrap items-center justify-between gap-4 text-sm">
      <div class="font-serif text-stone">
        {#if rateLimit?.remaining !== undefined}
          <span>{formatNumber(rateLimit.remaining)} requests left in the current window.</span>
        {:else}
          <span>{HOME.form.rateLimitFallback}</span>
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
    {#each HOME.features as feature, i (feature.heading)}
      <div class="text-center">
        <div class="mx-auto mb-4 flex h-12 w-12 items-center justify-center border-2 border-ink">
          <span class="flex items-center">
            {#if i === 0}
              <i class="ml-1 i-tabler-bolt h-6 w-6 text-yellow-500"></i>
            {:else if i === 1}
              <i class="ml-1 i-tabler-book h-6 w-6 text-blue-500"></i>
            {:else}
              <i class="ml-1 i-tabler-api h-6 w-6 text-green-500"></i>
            {/if}
          </span>
        </div>
        <h2 class="mb-2 font-semibold text-ink">{feature.heading}</h2>
        <p class="font-serif text-sm text-stone">{feature.body}</p>
      </div>
    {/each}
  </section>

  <section class="mt-20 border-t border-mist pt-12">
    <div class="mb-8 flex items-center justify-between gap-4">
      <div>
        <p class="muted-label mb-2">{HOME.recent.label}</p>
        <h2 class="font-serif text-2xl font-semibold text-ink">{HOME.recent.heading}</h2>
      </div>
      <a
        class="border-b border-stone text-sm font-medium text-stone hover:border-ink hover:text-ink"
        href={resolve('/library')}>
        {HOME.recent.viewAll}
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
                  {item.excerpt || HOME.recent.excerptFallback}
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
          {recentError || HOME.recent.empty}
        </p>
      </div>
    {/if}
  </section>
</main>

{#if errorMessage}
  <div class="fixed inset-x-0 bottom-6 z-50 mx-auto max-w-xl px-6">
    <div class="rounded-2xl border border-red-200 bg-white px-5 py-4 shadow-xl">
      <p class="muted-label mb-2 text-red-700">{HOME.errors.label}</p>
      <p class="font-serif text-sm text-graphite">{errorMessage}</p>
    </div>
  </div>
{/if}
