# How It Works

Lectito follows the same broad approach as Mozilla Readability.

The extractor starts with a full HTML document and tries to find the subtree
that behaves like an article. It uses signals that tend to survive across sites:
text length, paragraph density, semantic tags, class and id names, and the ratio
of links to readable text.

1. Parse the document.
2. Recover useful content from common snapshots, including selected mobile and
   shadow-root cases.
3. Extract metadata.
4. Try a matching site profile or code extractor when one applies.
5. Remove scripts, styles, hidden nodes, and unlikely content.
6. Score candidate content roots by text length, tag type, class/id hints, and
   link density.
7. Select the best root and include useful siblings.
8. Clean the selected content.
9. Apply schema text fallback when structured data is clearly better.
10. Return HTML, Markdown, text, and diagnostics.

Extraction runs several attempts. Later attempts relax cleanup rules when the
first pass produces too little text. The first attempt that reaches
`char_threshold` is accepted. If no attempt reaches the threshold, Lectito may
return the best non-empty attempt.

This retry model matters because pages fail in different ways. Some pages hide
the useful content behind classes that look like chrome. Others include enough
related links or widgets to pull the score away from the main text. Relaxed
attempts give Lectito another chance without making the first pass too loose.

`content_selector` can short-circuit root selection for known documents:

```rust
let options = ReadabilityOptions {
    content_selector: Some("main article".to_string()),
    ..ReadabilityOptions::default()
};
```

Site profiles provide URL-scoped hints without disabling generic extraction:

```rust
let options = ReadabilityOptions {
    site_profiles: vec![r#"
        name = "example"
        hosts = ["example.com"]
        content_roots = ["article"]
        remove = [".ad", "nav"]
    "#.to_string()],
    ..ReadabilityOptions::default()
};
```

If a profile produces content below `char_threshold`, Lectito records the
profile decision in diagnostics and continues with generic readability attempts.

After the root is selected, cleanup removes empty nodes, normalizes links and
media, preserves selected classes, and prepares the HTML for Markdown and text
conversion.
