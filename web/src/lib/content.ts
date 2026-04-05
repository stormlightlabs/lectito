export const SITE = {
  name: 'LECTITO',
  tagline: 'Extract readable content from any web page',
  footerTagline: 'A free reading utility for extracting the signal from cluttered pages.',
  copyright: '© 2026 Stormlight Labs. Open source under MIT.'
};

export const NAV = {
  extract: 'Extract',
  library: 'Library',
  about: 'About',
  api: 'API Docs',
  crate: 'Rust Crate',
  book: 'Book'
};

export const HOME = {
  meta: {
    title: 'Lectito',
    description: 'Extract clean, readable articles from any URL and browse cached reads in Lectito.'
  },
  hero: {
    label: 'Fast Rust extraction for research, archives, and reading queues',
    heading: 'Paste a URL to extract',
    body: 'Clean, readable articles from any web page.'
  },
  form: {
    urlLabel: 'Article URL',
    readerHint: 'Reader view opens after extraction and keeps the cached article on hand for later.',
    submitIdle: 'Extract Content',
    submitLoading: 'Extracting',
    rateLimitFallback: 'Free public API with request limits surfaced on every response.'
  },
  features: [
    {
      heading: 'Fast',
      body: 'Server-side caching and Rust extraction keep repeat reads responsive without sacrificing detail.'
    },
    {
      heading: 'Clean',
      body: 'Focused reader layouts, metadata capture, and multiple export formats make the result immediately usable.'
    },
    {
      heading: 'Public API',
      body: 'POST and GET extraction endpoints, a cached library, and rate-limit headers for predictable integrations.'
    }
  ],
  recent: {
    label: 'Archive Preview',
    heading: 'Recently Extracted',
    viewAll: 'View Library →',
    empty: 'No cached articles yet. Extract one above to seed the library.',
    excerptFallback: 'No excerpt was captured for this cached article yet.'
  },
  errors: {
    invalidUrl: 'Enter a valid URL, including the protocol.',
    noCacheId: 'The article was extracted, but no cache id was returned for the reader route.',
    label: 'Extraction Error'
  }
};

export const LIBRARY = {
  meta: {
    title: 'Library · Lectito',
    description: 'Browse cached Lectito extractions by recency, popularity, or title.'
  },
  hero: { label: 'Cached articles from the extraction service', heading: 'Library' },
  empty: 'Adjust the filters or extract a fresh article to seed the archive.',
  excerptFallback: 'No excerpt was stored for this cached article.',
  domainStatsFallback: 'No domain stats yet.',
  error: 'Library Error'
};

export const ABOUT = {
  meta: {
    title: 'About · Lectito',
    description: 'About Lectito, its API surface, rate limits, and example extraction workflows.'
  },
  about: {
    heading: 'About Lectito',
    paragraphs: [
      'Lectito is a free, open-source service for extracting readable content from web pages. It removes clutter, isolates the main article, and returns clean output as Markdown, HTML, plain text, or JSON.',
      'The stack pairs a Rust extraction engine with an Axum API and a Svelte client. Cached results back the library view, keeping frequent reads fast and making earlier extractions easy to revisit.',
      'The current public API is designed for research workflows, reading queues, and lightweight automation where consistent output matters more than scraping raw page chrome.'
    ]
  },
  features: {
    heading: 'Features',
    items: [
      {
        heading: 'Smart Content Extraction',
        body: 'Removes ads, sidebars, navigation, and other low-value blocks so the article body is easier to read or reprocess.'
      },
      {
        heading: 'Multiple Output Formats',
        body: 'Request HTML, Markdown, text, or JSON depending on whether you need reader mode, export, or structured ingestion.'
      },
      {
        heading: 'Cached Library',
        body: 'Previously extracted articles are kept in Postgres, powering pagination, search, popularity sorting, and revisit flows.'
      },
      {
        heading: 'Operational Controls',
        body: 'Rate limiting, blocklists, and admin endpoints give the server enough leverage to stay public-facing without being reckless.'
      }
    ]
  },
  rateLimits: {
    heading: 'Rate Limits',
    intro:
      'The API enforces per-IP limits across three windows so the public service stays stable under mixed interactive and scripted traffic.'
  },
  apiReference: { heading: 'API Reference' }
};

export const READER = {
  unavailable: {
    label: 'Reader unavailable',
    body: 'That cached article could not be loaded.',
    back: 'Back to Library'
  },
  nav: { extractAnother: '← Extract Another', browseLibrary: 'Browse Library →' },
  authorFallback: 'Unknown author'
};
