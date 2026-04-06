# Scoring Algorithm

Detailed explanation of how Lectito scores HTML elements to identify article content.

## Overview

The scoring algorithm assigns a numeric score to each HTML element, indicating how likely it is to contain the main article content. Higher scores indicate better content candidates.

The exact weights evolve as the extractor improves, so treat this page as a guide to the scoring logic, not a frozen ABI.

## Score Formula

At a high level, the score still looks like this:

```text
element_score = (base_tag_score
               + class_id_weight
               + content_density_score
               + container_bonus)
               × (1 - link_density)
```

## Base Tag Score

Different HTML tags have different inherent scores, reflecting their likelihood of containing content:

| Tag            | Typical Bias      | Rationale                                 |
| -------------- | ----------------- | ----------------------------------------- |
| `<article>`    | Positive          | Semantic article container                |
| `<section>`    | Positive          | Logical content section                   |
| `<div>`        | Positive          | Generic container, often used for content |
| `<blockquote>` | Slightly positive | Quoted content                            |
| `<pre>`        | Neutral           | Preformatted text                         |
| `<header>`     | Negative          | Header, not main content                  |
| `<footer>`     | Negative          | Footer, not main content                  |
| `<nav>`        | Negative          | Navigation                                |
| `<form>`       | Negative          | Forms, not content                        |

## Class/ID Weight

Class and ID attributes strongly indicate element purpose.

Positive patterns bias the scorer toward article-like containers. Negative patterns bias it away from sidebars, menus, comments, related-story blocks, and similar chrome.

Examples:

- Positive: `class="article-content"`, `id="main-content"`
- Negative: `class="sidebar"`, `id="footer"`, `class="navigation"`

## Content Density Score

The scorer rewards elements with substantial text content:

- more readable text
- more punctuation and sentence structure
- less boilerplate

Real article content tends to have more continuous prose than navigation or metadata.

## Link Density Penalty

Nodes packed with links are usually navigation, metadata, or related-story rails, not the article body.

```text
link_density = linked_text / total_text
```

Higher link density reduces the final score.

## Branch-Specific Heuristics

The current branch adds a few important refinements on top of the classic Readability-style score:

### Entry-Point Bias

Common article containers such as `article`, `main`, and well-known content wrappers get an early structural advantage before raw text density decides the winner.

### Sibling Aggregation

When several nearby candidates score well, Lectito can walk upward and treat them as one article body instead of picking only a single subtree.

### Table Handling

Layout tables and data tables are treated differently. Data tables should survive extraction. Layout tables should not dominate it.

### Retry Strategy

If the first pass extracts too little text, Lectito retries with progressively looser settings before it gives up.

## Thresholds

Two thresholds still matter most:

### Score Threshold

Minimum score for extraction.

If no element scores high enough, extraction fails with `LectitoError::NotReadable`.

### Character Threshold

Minimum character count for meaningful content.

Even with a strong score, content must still be large enough to count as readable.

## Scoring Edge Cases

### Empty Elements

Elements with no text receive a negligible score and are ignored.

### Nested Elements

Both parent and child elements are scored. The best candidate can appear at any level of the tree.

### Sibling Elements

Adjacent elements with similar scores may be grouped as part of the same article.

### Negative Scores

Elements that look like navigation or chrome can end up with negative scores and fall out of contention.

## Configuration Affecting Scoring

Adjust scoring behavior with `ReadabilityConfig`:

- `min_score`
- `char_threshold`
- `nb_top_candidates`
- `max_elems_to_parse`
- `remove_unlikely`

See [Configuration](../library/configuration.md) for details.
