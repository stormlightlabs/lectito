# Configuration

Customize Lectito's extraction behavior with configuration options.

## ReadabilityConfig

The `ReadabilityConfig` struct controls extraction parameters. Use the builder pattern:

```rs
use lectito_core::{Readability, ReadabilityConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ReadabilityConfig::builder()
        .min_score(25.0)
        .char_threshold(500)
        .nb_top_candidates(8)
        .preserve_images(true)
        .preserve_video_embeds(true)
        .build();

    let reader = Readability::with_config(config);
    let article = reader.parse("<html>...</html>")?;

    Ok(())
}
```

## Readability Options

| Field | Default | Description |
| ----- | ------- | ----------- |
| `min_score` | `20.0` | Minimum score required for extraction |
| `char_threshold` | `500` | Minimum character count for strong candidates |
| `nb_top_candidates` | `5` | Number of top candidates to keep during scoring |
| `max_elems_to_parse` | `0` | Maximum number of elements to score, `0` means unlimited |
| `remove_unlikely` | `true` | Remove obvious chrome before scoring |
| `keep_classes` | `false` | Preserve class attributes in output HTML |
| `preserve_images` | `true` | Keep images in extracted content |
| `preserve_video_embeds` | `true` | Keep supported video embeds |

### Strict Extraction

For high-quality content only:

```rs
use lectito_core::ReadabilityConfig;

let config = ReadabilityConfig::builder()
    .min_score(30.0)
    .char_threshold(1000)
    .build();
```

### Lenient Extraction

For short pages or difficult layouts:

```rs
use lectito_core::ReadabilityConfig;

let config = ReadabilityConfig::builder()
    .min_score(10.0)
    .char_threshold(200)
    .remove_unlikely(false)
    .build();
```

### Text-Only Extraction

Remove images and embeds:

```rs
use lectito_core::ReadabilityConfig;

let config = ReadabilityConfig::builder()
    .preserve_images(false)
    .preserve_video_embeds(false)
    .build();
```

## FetchConfig

Configure HTTP fetching behavior:

```rs
use lectito_core::{fetch_and_parse_with_config, FetchConfig, ReadabilityConfig};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fetch_config = FetchConfig {
        timeout: 60,
        user_agent: "MyBot/1.0".to_string(),
        headers: HashMap::new(),
    };

    let read_config = ReadabilityConfig::builder()
        .min_score(25.0)
        .build();

    let article = fetch_and_parse_with_config(
        "https://example.com/article",
        &read_config,
        &fetch_config,
    ).await?;

    Ok(())
}
```

### Fetch Options

| Field | Type | Default | Description |
| ----- | ---- | ------- | ----------- |
| `timeout` | `u64` | `30` | Request timeout in seconds |
| `user_agent` | `String` | Browser-like Lectito UA | User-Agent header value |
| `headers` | `HashMap<String, String>` | empty | Extra request headers |

## Default Values

```rs
use lectito_core::ReadabilityConfig;

let config = ReadabilityConfig::default();

assert_eq!(config.min_score, 20.0);
assert_eq!(config.char_threshold, 500);
assert_eq!(config.nb_top_candidates, 5);
assert_eq!(config.max_elems_to_parse, 0);
assert!(config.remove_unlikely);
assert!(!config.keep_classes);
assert!(config.preserve_images);
assert!(config.preserve_video_embeds);
```

## Site Configuration

For sites that require custom extraction rules, use the site configuration feature:

```toml
[dependencies]
lectito-core = { version = "0.1", features = ["siteconfig"] }
```

Site configuration uses the FTR-style ruleset and the `ConfigLoader` APIs to apply per-site extraction rules.

## Next Steps

- [Async vs Sync](async-vs-sync.md) - Understanding async APIs
- [Output Formats](output-formats.md) - Detailed format documentation
- [Scoring Algorithm](../concepts/scoring-algorithm.md) - How scores are calculated
