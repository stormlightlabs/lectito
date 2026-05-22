# Site Profiles

Site profiles are TOML extraction hints scoped by URL host. They are useful when
a site has a stable content container or predictable clutter, but still returns
ordinary article-shaped HTML.

Profiles run before generic readability scoring. If a profile produces text
below `char_threshold`, Lectito records the profile decision in diagnostics and
continues with generic extraction.

## Example

```toml
name = "example"
hosts = ["example.com"]
subdomains = true
path_prefixes = ["/blog"]
exclude_path_prefixes = ["/blog/comments"]
content_roots = ["article", "#content"]
remove = [".ad", "nav", "footer"]
remove_id_or_class = ["sidebar"]

[metadata]
title = ["h1"]
author = [".byline"]
date = ["time/@datetime"]
image = ["meta[property='og:image']/@content"]
site_name = "Example"
title_suffixes = [" - Example"]

[cleanup]
enabled = true
prune = true

[fallback]
generic_on_empty = true
```

## Fields

| Field | Meaning |
| --- | --- |
| `name` | Human-readable profile name used in diagnostics. |
| `hosts` | Hosts matched by the profile. `www.` is ignored during matching. |
| `subdomains` | When true, subdomains of each host also match. |
| `path_prefixes` | Optional path prefixes. Omit to match every path on the host. |
| `exclude_path_prefixes` | Optional path prefixes that suppress the profile after host matching. |
| `content_roots` | CSS selectors or supported XPath selectors for article roots. |
| `remove` | CSS selectors or supported XPath selectors to remove before extraction. |
| `remove_id_or_class` | Exact id or class tokens to remove. |

Metadata fields are optional selector lists, except `site_name`, which is a
constant. Selectors may target attributes with the supported XPath `.../@attr`
form.

Cleanup defaults to enabled. `prune` controls conditional cleanup. Disabling
cleanup should be reserved for sites where the profile root is already clean and
generic cleanup removes useful structure.

## Selector Support

Profiles accept CSS selectors directly. They also accept a focused XPath subset
for compatibility with rule corpuses and older bundled rules:

- `//tag`
- `//*[@id='value']`
- `//tag[@class='a b']`
- `//tag[contains(@class, 'value')]`
- `/text()` suffixes
- `/@attribute` suffixes for metadata selectors

Unsupported XPath expressions are ignored by selector matching, so bundled
profiles should have tests that prove their roots match representative pages.

## User Profiles

Rust callers pass profile TOML strings through `ReadabilityOptions`:

```rust
let options = ReadabilityOptions {
    site_profiles: vec![std::fs::read_to_string("example.com.toml")?],
    ..ReadabilityOptions::default()
};
```

The CLI accepts repeatable profile paths:

```sh
lectito parse article.html --url https://example.com/post --site-profile example.com.toml
```

User profiles take precedence over bundled profiles. More specific host and path
matches win within each source group.
