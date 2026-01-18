# Scoring Algorithm

Detailed explanation of how Lectito scores HTML elements to identify article content.

## Overview

The scoring algorithm assigns a numeric score to each HTML element, indicating how likely it is to contain the main article content. Higher scores indicate better content candidates.

## Score Formula

The final score for each element is calculated as:

```text
element_score = (base_tag_score
               + class_id_weight
               + content_density_score)
               × (1 - link_density)
```

Let's break down each component.

## Base Tag Score

Different HTML tags have different inherent scores, reflecting their likelihood of containing content:

| Tag            | Score | Rationale                                 |
| -------------- | ----- | ----------------------------------------- |
| `<article>`    | +10   | Semantic article container                |
| `<section>`    | +8    | Logical content section                   |
| `<div>`        | +5    | Generic container, often used for content |
| `<blockquote>` | +5    | Quoted content                            |
| `<pre>`        | +5    | Preformatted text                         |
| `<td>`         | +3    | Table cell                                |
| `<p>`          | +3    | Paragraph                                 |
| `<th>`         | +3    | Table header                              |
| `<ul>`/`<ol>`  | +3    | Lists                                     |
| `<address>`    | -3    | Contact info, unlikely to be main content |
| `<h1>`-`<h6>`  | -0.5  | Headings, not content themselves          |
| `<form>`       | -3    | Forms, not content                        |
| `<li>`         | -1    | List items, lower than container          |

## Class/ID Weight

Class and ID attributes strongly indicate element purpose:

### Positive Patterns

These patterns indicate content elements:

```regex
(?i)(article|body|content|entry|hentry|h-entry|main|page|post|text|blog|story)
```

**Weight**: +25 points

Examples:

- `class="article-content"`
- `id="main-content"`
- `class="post-body"`

### Negative Patterns

These patterns indicate non-content elements:

```text
(?i)(banner|breadcrumbs?|combx|comment|community|disqus|extra|foot|header|menu|related|remark|rss|shoutbox|sidebar|sponsor|ad-break|agegate|pagination|pager|popup)
```

**Weight**: -25 points

Examples:

- `class="sidebar"`
- `id="footer"`
- `class="navigation"`

## Content Density Score

Rewards elements with substantial text content:

### Character Density

1 point per 100 characters, maximum 3 points.

```rs
char_score = (text_length / 100).min(3)
```

### Punctuation Density

1 point per 5 commas/periods, maximum 3 points.

```rs
punct_score = (comma_count / 5).min(3)
```

Total content density:

```rs
content_density = char_score + punct_score
```

**Rationale**: Real article content has more text and punctuation than navigation or metadata.

## Link Density Penalty

Penalizes elements with too many links:

```text
link_density = (length of all <a> tag text) / (total text length)
final_score = raw_score × (1 - link_density)
```

**Examples**:

- Text "Click here": link density = 100% (10/10)
- Text "See the [article](link) for details": link density = 33% (7/21)
- Text "Article content with no links": link density = 0%

**Rationale**: Navigation menus, lists of links, and metadata have high link density. Real content has low link density.

## Complete Example

Consider this HTML:

```html
<div class="article-content">
    <h1>Article Title</h1>
    <p>
        This is a substantial paragraph with plenty of text, including multiple
        sentences, and commas, to demonstrate how content density scoring works.
    </p>
    <p>
        Another paragraph with even more text, details, and information to
        increase the character count.
    </p>
</div>
```

### Step-by-Step Scoring

#### 1 Base Tag Score

`<div>`: +5

#### 2 Class/ID Weight

`class="article-content"` contains "article" and "content": +25

#### 3 Content Density

- Text length: ~220 characters
- Character score: min(220/100, 3) = 2
- Commas: 4
- Punctuation score: min(4/5, 3) = 0
- Total: 2 points

#### 4 Link Density

No links: link density = 0

#### 5 Final Score

```rs
(5 + 25 + 2) × (1 - 0) = 32
```

This element would score 32, well above the default threshold of 20.

## Thresholds

Two thresholds determine if content is readable:

### Score Threshold

Minimum score for extraction (default: 20.0).

If no element scores above this, extraction fails with `LectitoError::NotReaderable`.

### Character Threshold

Minimum character count (default: 500).

Even with high score, content must have enough text to be meaningful.

## Scoring Edge Cases

### Empty Elements

Elements with no text receive score of 0 and are ignored.

### Nested Elements

Both parent and child elements are scored. The highest-scoring element at any level is selected.

### Sibling Elements

Adjacent elements with similar scores may be grouped as part of the same article.

### Negative Scores

Elements can have negative scores (e.g., navigation). They're excluded from selection.

## Configuration Affecting Scoring

Adjust scoring behavior with `ReadabilityConfig`:

```rs
use lectito_core::ReadabilityConfig;

let config = ReadabilityConfig::builder()
    .min_score(25.0)           // Higher threshold
    .char_threshold(1000)      // Require more content
    .min_content_length(200)   // Longer minimum text
    .build();
```

See [Configuration](../library/configuration.md) for details.

## Practical Implications

### Why Articles Score Well

- Semantic tags (`<article>`)
- Descriptive classes (`article-content`)
- Substantial text (high character count)
- Punctuation (commas, periods)
- Few links (low link density)

### Why Navigation Scores Poorly

- Generic or negative classes (`sidebar`, `navigation`)
- Little text (just link labels)
- Many links (high link density)
- Short content (fails character threshold)

### Why Comments May Score Poorly

- Often in negative classed containers (`comments`)
- Short individual comments
- Many links (usernames, replies)
- Variable quality

## Site Configuration

When automatic scoring fails, provide XPath rules:

```toml
# example.com.toml
[[fingerprints]]
pattern = "example.com"

[[fingerprints.extract]]
title = "//h1[@class='article-title']"
content = "//div[@class='article-body']"
```

See [Configuration](../library/configuration.md) for details.

## References

- Original Readability.js: [Mozilla Readability](https://github.com/mozilla/readability)
- Algorithm inspiration: [Arc90 Readability](https://code.google.com/archive/p/arc90labs-readability/)

## Next Steps

- [How It Works](how-it-works.md) - Overall extraction pipeline
- [Configuration](../library/configuration.md) - Customizing behavior
- [Basic Usage](../library/basic-usage.md) - Using the API
