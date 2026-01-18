# How It Works

Understanding the Lectito content extraction pipeline.

## Overview

Lectito implements a content extraction algorithm inspired by Mozilla's Readability.js. The algorithm identifies the main article content by analyzing the HTML structure, scoring elements based on various heuristics, and selecting the highest-scoring content.

## Extraction Pipeline

The extraction process consists of four main stages:

```text
HTML Input → Preprocessing → Scoring → Selection → Post-processing → Article
```

### 1. Preprocessing

Clean the HTML to improve scoring accuracy:

- Remove unlikely content: scripts, styles, iframes, and hidden nodes
- Strip elements with unlikely class/ID patterns
- Preserve structure: maintain HTML hierarchy for accurate scoring

**Why**: Preprocessing removes elements that could confuse the scoring algorithm or contain non-article content.

### 2. Scoring

Score each element based on content characteristics:

- **Tag score**: Different HTML tags have different base scores
- **Class/ID weight**: Positive patterns (article, content) vs negative (sidebar, footer)
- **Content density**: Length and punctuation indicate content quality
- **Link density**: Too many links suggests navigation/metadata, not content

**Why**: Scoring identifies which elements are most likely to contain the main article content.

### 3. Selection

Select the highest-scoring element as the article candidate:

- Find element with highest score (bias toward semantic containers when scores are close)
- Check if score meets minimum threshold (default: 20.0)
- Check if content length meets minimum threshold (default: 500 chars)
- Return error if content doesn't meet thresholds

**Why**: Selection ensures we extract actual article content, not navigation or ads.

### 4. Post-processing

Clean up the selected content:

- Include sibling elements: adjacent content blocks and shared-parent headers
- Remove remaining clutter: ads, comments, social widgets
- Clean up whitespace: normalize spacing and formatting
- Preserve structure: maintain headings, paragraphs, lists

**Why**: Post-processing improves the quality of extracted content and includes related elements.

## Data Flow

```text
Input HTML
    ↓
parse_to_document()
    ↓
preprocess_html() → Cleaned HTML
    ↓
build_dom_tree() → DOM Tree
    ↓
calculate_score() → Scored Elements
    ↓
extract_content() → Selected Element
    ↓
postprocess_html() → Cleaned Content
    ↓
extract_metadata() → Metadata
    ↓
Article
```

## Key Components

### Document and Element

The `Document` and `Element` types wrap the `scraper` crate's HTML parsing:

```rs
use lectito_core::{Document, Element};

let doc = Document::parse(html)?;
let elements: Vec<Element> = doc.select("article p")?;
```

These provide a convenient API for DOM manipulation and element traversal.

### Scoring Algorithm

The scoring algorithm combines multiple factors:

```text
element_score = base_tag_score
              + class_id_weight
              + content_density_score
              × (1 - link_density)
```

See [Scoring Algorithm](scoring-algorithm.md) for details.

### Metadata Extraction

Separate process extracts metadata from the HTML:

- **Title**: `<h1>`, `<title>`, or Open Graph tags
- **Author**: meta tags, bylines, schema.org
- **Date**: meta tags, time elements, schema.org
- **Excerpt**: meta description, first paragraph

## Why This Approach

### Content Over Structure

Unlike XPath-based extraction, Lectito doesn't rely on fixed HTML structures. It analyzes content characteristics, making it work across many sites without custom rules.

### Heuristic-Based

The algorithm uses heuristics (rules of thumb) derived from analyzing thousands of articles. This makes it flexible and adaptable to different site designs.

### Fallback Mechanism

For sites where the algorithm fails, Lectito supports site-specific configuration files with XPath expressions. See [Configuration](../library/configuration.md) for details.

## Limitations

### Sites That May Fail

- Very short pages (tweets, status updates)
- Non-article content (product pages, search results)
- Unusual layouts (some single-column designs)
- Heavily JavaScript-dependent content

### Improving Extraction

For difficult sites:

1. **Adjust thresholds**: Lower `min_score` or `char_threshold`
2. **Site configuration**: Provide XPath rules
3. **Manual curation**: Use XPath or CSS selectors directly

See [Configuration](../library/configuration.md) for options.

## Comparison to Alternatives

| Approach             | Pros                                            | Cons                              |
| -------------------- | ----------------------------------------------- | --------------------------------- |
| **Lectito**          | Works across many sites, no custom rules needed | May fail on unusual layouts       |
| **XPath**            | Precise, predictable                            | Requires custom rules per site    |
| **CSS Selectors**    | Simple, familiar                                | Brittle, breaks on layout changes |
| **Machine Learning** | Adaptable                                       | Complex, requires training data   |

Lectito strikes a balance: works well for most sites without custom rules, with site configuration as a fallback.

## Performance Considerations

- **Parsing**: HTML parsing is fast but not instant
- **Scoring**: Traverses entire DOM, O(n) complexity
- **Fetching**: Async for non-blocking I/O
- **Memory**: Entire document loaded into memory

For large-scale extraction, consider batching and concurrent fetches.

## Next Steps

- [Scoring Algorithm](scoring-algorithm.md) - Detailed scoring explanation
- [Configuration](../library/configuration.md) - Customizing extraction
- [Basic Usage](../library/basic-usage.md) - Using the API
