# CHANGELOG

## Unreleased

### [2026-01-17]

- Refactored the library API with a unified `Article` struct and a builder-based
configuration system for flexible content extraction.
- Expanded output with plain text formatter and integrated article methods
- Site configuration system using the FTR format and XPath 1.0 for more precise content
extraction and advanced document filtering.
- Automated CMS fingerprinting and domain-based rule loading with fallback to
heuristic-based autodetection.
- QA suite including performance benchmarks, integration tests, and a set of site
fixtures.

### [2026-01-16]

- Clean-up pipeline to remove empty nodes, conditional comments, and redundant
formatting while fixing relative URLs.
- Markdown output support with automatic TOML frontmatter generation, GFM table
conversion, and optional reference link tables.
- JSON output format and metadata-only extraction mode with integrated language
detection.

- Content candidate id and scoring system that tracks top candidates and propagates
scores to parent and grandparent elements
- Sibling selection to include related content based on score thresholds and link
density analysis
- CLI options (--char-threshold and --max-elements) for fine-tuning content extraction
behavior

- Content scoring system using base tag weights, class/ID pattern matching, and content
density analysis
- Link density calculation and penalty system to penalize navigation-heavy elements and
favor prose-rich content
- Combined scoring function that evaluates elements based on multiple signals including
character count, comma frequency, and link-to-text ratios

- Metadata extraction system with fallback priority chains for title, author, date,
excerpt, and site name
- JSON-LD parser module to extract structured data from web pages
- Word count and reading time calculations for extracted articles

- HTML preprocessing pipeline that removes scripts, styles, comments, and hidden
elements
- Pattern-based filtering to remove unlikely content candidates (banners, sidebars, ads)
while preserving positive candidates (articles, main content)

### [2026-01-15]

- Multi-source input handling supporting URLs (HTTP fetch), local files, and stdin
- Basic DOM parsing infrastructure using scraper with configurable timeouts and
user-agent settings
