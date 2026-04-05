<script lang="ts">
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import { NAV, SITE } from '$lib/content';

  type NavKey = 'home' | 'library' | 'about' | 'docs';

  const links: { href: `/${'' | 'library' | 'about'}`; label: string; key: NavKey }[] = [
    { href: '/', label: NAV.extract, key: 'home' },
    { href: '/library', label: NAV.library, key: 'library' },
    { href: '/about', label: NAV.about, key: 'about' }
  ];

  const active = $derived.by((): NavKey | undefined => {
    const path = page.url.pathname;
    if (path === '/') return 'home';
    if (path.startsWith('/library')) return 'library';
    if (path.startsWith('/about')) return 'about';
    if (path.startsWith('/docs')) return 'docs';
    return undefined;
  });
</script>

<header class="bg-[rgba(250,250,250,0.84)] backdrop-blur-sm">
  <div class="mx-auto max-w-6xl px-6 py-6">
    <div class="mb-4 flex items-center justify-between gap-6">
      <nav class="flex flex-wrap items-center gap-6 text-sm text-stone">
        {#each links as link (`${link.href}:${link.key}`)}
          <a
            aria-current={active === link.key ? 'page' : undefined}
            class={`border-b pb-0.5 ${
              active === link.key ? 'border-ink text-ink' : 'border-transparent hover:border-ink hover:text-ink'
            }`}
            href={resolve(link.href)}>
            {link.label}
          </a>
        {/each}
        <a
          aria-current={active === 'docs' ? 'page' : undefined}
          class={`border-b pb-0.5 ${
            active === 'docs' ? 'border-ink text-ink' : 'border-transparent hover:border-ink hover:text-ink'
          }`}
          href={resolve('/docs')}>
          {NAV.api}
        </a>
      </nav>
    </div>
    <div class="border-b-2 border-ink pb-6">
      <a class="block text-center" href={resolve('/')}>
        <span class="block text-5xl font-bold tracking-tight text-ink md:text-6xl">{SITE.name}</span>
        <span class="mt-2 block font-serif text-sm text-stone italic"> {SITE.tagline} </span>
      </a>
    </div>
  </div>
</header>
