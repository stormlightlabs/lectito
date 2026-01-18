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
        .preserve_images(true)
        .build();

    let reader = Readability::with_config(config);
    let article = reader.parse("<html>...</html>")?;

    Ok(())
}
```

## Configuration Options

### min_score

Minimum readability score for content to be considered extractable (default: 20.0).

```rs
use lectito_core::ReadabilityConfig;

let config = ReadabilityConfig::builder()
    .min_score(25.0)
    .build();
```

Higher values are more strict. If content scores below this threshold, parsing returns `LectitoError::NotReaderable`.

### char_threshold

Minimum character count for content to be considered (default: 500).

```rs
use lectito_core::ReadabilityConfig;

let config = ReadabilityConfig::builder()
    .char_threshold(1000)
    .build();
```

Increase this for short pages or blog posts to avoid extracting navigation elements.

### preserve_images

Whether to preserve images in the extracted content (default: true).

```rs
use lectito_core::ReadabilityConfig;

let config = ReadabilityConfig::builder()
    .preserve_images(false)
    .build();
```

### min_content_length

Minimum length for text content (default: 140).

```rs
use lectito_core::ReadabilityConfig;

let config = ReadabilityConfig::builder()
    .min_content_length(200)
    .build();
```

### min_score_threshold

Threshold for minimum score during scoring (default: 20.0).

```rs
use lectito_core::ReadabilityConfig;

let config = ReadabilityConfig::builder()
    .min_score_threshold(25.0)
    .build();
```

## FetchConfig

Configure HTTP fetching behavior:

```rs
use lectito_core::{fetch_and_parse_with_config, FetchConfig, ReadabilityConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fetch_config = FetchConfig {
        timeout: 60,
        user_agent: "MyBot/1.0".to_string(),
        ..Default::default()
    };

    let read_config = ReadabilityConfig::builder()
        .min_score(25.0)
        .build();

    let article = fetch_and_parse_with_config(
        "https://example.com/article",
        &fetch_config,
        &read_config
    ).await?;

    Ok(())
}
```

### FetchConfig Options

| Field        | Type     | Default       | Description                |
| ------------ | -------- | ------------- | -------------------------- |
| `timeout`    | `u64`    | 30            | Request timeout in seconds |
| `user_agent` | `String` | "Lectito/..." | User-Agent header value    |

## Default Values

```rs
impl Default for ReadabilityConfig {
    fn default() -> Self {
        Self {
            min_score: 20.0,
            char_threshold: 500,
            preserve_images: true,
            min_content_length: 140,
            min_score_threshold: 20.0,
        }
    }
}
```

## Configuration Examples

### Strict Extraction

For high-quality content only:

```rs
use lectito_core::ReadabilityConfig;

let config = ReadabilityConfig::builder()
    .min_score(30.0)
    .char_threshold(1000)
    .min_content_length(300)
    .build();
```

### Lenient Extraction

For extracting from short pages:

```rs
use lectito_core::ReadabilityConfig;

let config = ReadabilityConfig::builder()
    .min_score(10.0)
    .char_threshold(200)
    .min_content_length(50)
    .build();
```

### Text-Only Extraction

Remove images and multimedia:

```rs
use lectito_core::ReadabilityConfig;

let config = ReadabilityConfig::builder()
    .preserve_images(false)
    .build();
```

### Custom Fetch Settings

Long timeout with custom user agent:

```rs
use lectito_core::FetchConfig;

let config = FetchConfig {
    timeout: 120,
    user_agent: "MyBot/1.0 (+https://example.com/bot)".to_string(),
};
```

## Site Configuration

For sites that require custom extraction rules, use the site configuration feature (requires `siteconfig` feature):

```toml
[dependencies]
lectito-core = { version = "0.1", features = ["siteconfig"] }
```

Site configuration uses the FTR (Five Filters Text) format. See [How It Works](../concepts/how-it-works.md) for details on site-specific extraction.

## Next Steps

- [Async vs Sync](async-vs-sync.md) - Understanding async APIs
- [Output Formats](output-formats.md) - Detailed format documentation
- [Scoring Algorithm](../concepts/scoring-algorithm.md) - How scores are calculated
