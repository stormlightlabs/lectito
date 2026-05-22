# Configuration

`ReadabilityOptions` control extraction.

The defaults are conservative. They favor article pages with enough text to be
useful and avoid exposing internal scoring knobs unless they affect common
integration cases.

```rust
use lectito::ReadabilityOptions;

let options = ReadabilityOptions {
    char_threshold: 800,
    nb_top_candidates: 8,
    content_selector: Some("article".to_string()),
    site_profiles: Vec::new(),
    ..ReadabilityOptions::default()
};
```

Fields:

| Field                   |     Default | Meaning                                                |
| ----------------------- | ----------: | ------------------------------------------------------ |
| `max_elems_to_parse`    |      `None` | Reject documents above this element count.             |
| `nb_top_candidates`     |         `5` | Number of high-scoring candidates to consider.         |
| `char_threshold`        |       `500` | Minimum extracted text length for an accepted attempt. |
| `content_selector`      |      `None` | CSS selector to prefer as the content root.            |
| `site_profiles`         |        `[]` | TOML site profiles for host-scoped extraction hints.   |
| `mobile_viewport_width` | `Some(480)` | Width used by recovery rules for mobile snapshots.     |
| `classes_to_preserve`   |        `[]` | Class names kept during cleanup.                       |
| `keep_classes`          |     `false` | Keep all class attributes.                             |
| `disable_json_ld`       |     `false` | Skip JSON-LD metadata extraction.                      |
| `link_density_modifier` |       `0.0` | Adjust link-density cleanup tolerance.                 |

Prefer `content_selector` when you already know the page shape. It is clearer
than trying to tune scores around a stable document layout.

Use `site_profiles` when you want the same kind of override to apply by URL
host, or when you need removal selectors and metadata hints alongside content
roots. Profiles are attempted before generic scoring, but weak profile output
falls back to the generic extractor.

Use `max_elems_to_parse` as a guardrail for untrusted input. It rejects very
large documents before extraction work continues.

`ReadableOptions` controls `is_probably_readable`.

Lower `min_content_length` for short posts or documentation pages. Raise
`min_score` when you want the quick check to reject borderline pages.

```rust
use lectito::ReadableOptions;

let options = ReadableOptions {
    min_content_length: 140,
    min_score: 20.0,
};
```
