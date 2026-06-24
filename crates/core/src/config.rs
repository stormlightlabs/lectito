use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MediaRetention {
    /// Remove media from extracted content.
    None,
    /// Text-first reader mode; keep only media that survives generic cleanup.
    Conservative,
    /// Keep images and figures that appear to be part of the article body.
    #[default]
    Article,
    /// Keep all media that remains inside the selected article subtree.
    All,
}

impl MediaRetention {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Conservative => "conservative",
            Self::Article => "article",
            Self::All => "all",
        }
    }
}

impl fmt::Display for MediaRetention {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for MediaRetention {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "none" => Ok(Self::None),
            "conservative" => Ok(Self::Conservative),
            "article" => Ok(Self::Article),
            "all" => Ok(Self::All),
            other => Err(format!(
                "invalid media retention mode '{other}' (expected none, conservative, article, or all)"
            )),
        }
    }
}

/// Options for full article extraction.
///
/// Defaults are intended for article pages. Set only the fields that solve a
/// specific input problem.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReadabilityOptions {
    /// Reject documents above this element count before extraction work starts.
    pub max_elems_to_parse: Option<usize>,
    /// Number of high-scoring roots kept during generic candidate selection.
    pub nb_top_candidates: usize,
    /// Minimum extracted text length required to accept an attempt.
    pub char_threshold: usize,
    /// CSS selector for a known article root.
    ///
    /// This bypasses generic root scoring for that document. Cleanup, media
    /// handling, URL rewriting, Markdown conversion, and diagnostics still run.
    pub content_selector: Option<String>,
    /// TOML site profiles used as URL-scoped extraction hints.
    #[serde(default)]
    pub site_profiles: Vec<String>,
    /// Viewport width used to recover content hidden behind mobile CSS rules.
    pub mobile_viewport_width: Option<usize>,
    /// Class names kept during cleanup when `keep_classes` is false.
    pub classes_to_preserve: Vec<String>,
    /// Keep all class attributes in extracted HTML.
    pub keep_classes: bool,
    /// Disable JSON-LD metadata extraction and JSON-LD article-body extraction.
    pub disable_json_ld: bool,
    /// Adjust link-density cleanup tolerance.
    pub link_density_modifier: f32,
    /// Controls whether images, figures, and embeds survive cleanup.
    #[serde(default)]
    pub media_retention: MediaRetention,
}

impl Default for ReadabilityOptions {
    fn default() -> Self {
        Self {
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
    }
}

/// Options for the quick readability check.
///
/// These options affect [`crate::is_probably_readable`] only. They do not
/// change full extraction.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReadableOptions {
    /// Minimum text length for a block to count toward readability.
    pub min_content_length: usize,
    /// Minimum accumulated score required for a readable result.
    pub min_score: f32,
}

impl Default for ReadableOptions {
    fn default() -> Self {
        Self { min_content_length: 140, min_score: 20.0 }
    }
}

/// Options for Markdown-to-HTML rendering.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct MarkdownOptions {
    /// Enable GitHub Flavored Markdown extensions.
    pub gfm: bool,
    /// Enable footnote rendering.
    pub footnotes: bool,
    /// Enable math rendering.
    pub math: bool,
    /// Allow raw HTML in Markdown input.
    pub allow_raw_html: bool,
}

impl Default for MarkdownOptions {
    fn default() -> Self {
        Self { gfm: true, footnotes: true, math: true, allow_raw_html: false }
    }
}

/// Extracted article content and metadata.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Article {
    /// Best title found in metadata or content.
    pub title: Option<String>,
    /// Author or byline when available.
    pub byline: Option<String>,
    /// Text direction from document metadata.
    pub dir: Option<String>,
    /// Language from document metadata.
    pub lang: Option<String>,
    /// Cleaned article HTML wrapped in a Readability-compatible page div.
    pub content: String,
    /// Markdown version of `content`.
    pub markdown: String,
    /// Plain text extracted from `content`.
    pub text_content: String,
    /// UTF-16 text length, matching Mozilla Readability's length convention.
    pub length: usize,
    /// Summary from metadata or the first article paragraph.
    pub excerpt: Option<String>,
    /// Site or publisher name.
    pub site_name: Option<String>,
    /// Published timestamp from metadata when available.
    pub published_time: Option<String>,
    /// Lead image URL from metadata when available.
    pub image: Option<String>,
    /// Source domain inferred from the base URL.
    pub domain: Option<String>,
    /// Favicon URL from document metadata when available.
    pub favicon: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtractFlags {
    pub strip_unlikely: bool,
    pub weight_classes: bool,
    pub clean_conditionally: bool,
}

impl ExtractFlags {
    pub fn all() -> Self {
        Self { strip_unlikely: true, weight_classes: true, clean_conditionally: true }
    }
}
