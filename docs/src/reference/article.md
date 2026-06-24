# Article

`Article` is the extraction result.

The struct is serializable and contains both content and metadata. The content
fields are generated from the selected article root; metadata can come from
document metadata, JSON-LD, Open Graph tags, or the extracted content itself.

```rust
pub struct Article {
    pub title: Option<String>,
    pub byline: Option<String>,
    pub dir: Option<String>,
    pub lang: Option<String>,
    pub content: String,
    pub markdown: String,
    pub text_content: String,
    pub length: usize,
    pub excerpt: Option<String>,
    pub site_name: Option<String>,
    pub published_time: Option<String>,
    pub image: Option<String>,
    pub domain: Option<String>,
    pub favicon: Option<String>,
}
```

Fields:

| Field            | Meaning                                                        |
| ---------------- | -------------------------------------------------------------- |
| `title`          | Best title from metadata or document content.                  |
| `byline`         | Author/byline when detected.                                   |
| `dir`            | Text direction, such as `ltr` or `rtl`.                        |
| `lang`           | Document language when detected.                               |
| `content`        | Cleaned article HTML.                                          |
| `markdown`       | Markdown generated from `content`.                             |
| `text_content`   | Plain text generated from `content`.                           |
| `length`         | UTF-16 length of extracted text, matching Mozilla Readability. |
| `excerpt`        | Short summary or first useful paragraph.                       |
| `site_name`      | Publisher or site name.                                        |
| `published_time` | Publication timestamp when detected.                           |
| `image`          | Lead image URL when detected.                                  |
| `domain`         | Source domain when available.                                  |
| `favicon`        | Favicon URL when detected.                                     |

`content`, `markdown`, and `text_content` are different views of the same
extracted article. Prefer `content` when structure matters, `markdown` when the
article will be displayed or edited as text, and `text_content` when indexing or
summarizing.

`length` follows Mozilla Readability's UTF-16 convention. It can differ from a
Rust `chars().count()` value for text outside the Basic Multilingual Plane.
