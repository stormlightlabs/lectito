# Options

## ReadabilityOptions

`ReadabilityOptions` changes extraction behavior. Most callers should start
with `ReadabilityOptions::default()` and only set fields that solve a specific
problem.

```rust
pub struct ReadabilityOptions {
    pub max_elems_to_parse: Option<usize>,
    pub nb_top_candidates: usize,
    pub char_threshold: usize,
    pub content_selector: Option<String>,
    pub site_profiles: Vec<String>,
    pub mobile_viewport_width: Option<usize>,
    pub classes_to_preserve: Vec<String>,
    pub keep_classes: bool,
    pub disable_json_ld: bool,
    pub link_density_modifier: f32,
    pub media_retention: MediaRetention,
}

pub enum MediaRetention {
    None,
    Conservative,
    Article,
    All,
}
```

Defaults:

```rust
ReadabilityOptions {
    max_elems_to_parse: None,
    nb_top_candidates: 5,
    char_threshold: 500,
    content_selector: None,
    site_profiles: Vec::new(),
    mobile_viewport_width: Some(480),
    classes_to_preserve: Vec::new(),
    keep_classes: false,
    disable_json_ld: false,
    link_density_modifier: 0.0,
    media_retention: MediaRetention::Article,
}
```

`content_selector` is the most direct override. Use it when the caller knows
where the article lives in the document. When it is unset, Lectito still tries a
small built-in list of common article-body containers before generic scoring.

`site_profiles` accepts TOML profile strings that provide host-scoped content
roots, removal selectors, metadata hints, cleanup settings, and fallback
behavior. Profiles run before generic scoring, after the JSON-LD and known
container fast paths.

`char_threshold` controls when an attempt is accepted. `nb_top_candidates`
controls how many candidates remain in play during generic scoring.

`disable_json_ld` skips JSON-LD metadata extraction and the JSON-LD article-body
fast path. It does not disable Open Graph, Twitter card, or DOM metadata.

`media_retention` controls image and media preservation in the extracted article:

- `None`: remove figures, images, and embedded media from content.
- `Conservative`: text-first cleanup; media survives only if the generic extractor keeps it.
- `Article`: keep figures/images that look like article body content. This is the default.
- `All`: keep media that remains in the selected article subtree, subject to unsafe/embed cleanup.

## ReadableOptions

`ReadableOptions` only affects `is_probably_readable`. It does not change full
article extraction.

```rust
pub struct ReadableOptions {
    pub min_content_length: usize,
    pub min_score: f32,
}
```

Use lower thresholds for short-form content. Use higher thresholds when false
positives are more expensive than missed articles.

Defaults:

```rust
ReadableOptions {
    min_content_length: 140,
    min_score: 20.0,
}
```
