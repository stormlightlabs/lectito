use lectito::{MarkdownOptions, MediaRetention, ReadabilityOptions, ReadableOptions};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
enum MediaRetentionDto {
    None,
    Conservative,
    Article,
    All,
}

impl From<MediaRetentionDto> for MediaRetention {
    fn from(value: MediaRetentionDto) -> Self {
        match value {
            MediaRetentionDto::None => Self::None,
            MediaRetentionDto::Conservative => Self::Conservative,
            MediaRetentionDto::Article => Self::Article,
            MediaRetentionDto::All => Self::All,
        }
    }
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub ok: bool,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExtractRequest {
    pub url: String,
    #[serde(default)]
    pub options: Option<ReadabilityOptionsDto>,
    #[serde(default)]
    pub diagnostics: bool,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExtractResponse {
    pub article: Option<ArticleDto>,
    #[schema(value_type = Option<Object>)]
    pub diagnostics: Option<serde_json::Value>,
    #[schema(value_type = u64)]
    pub elapsed_ms: u128,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateRequest {
    pub url: String,
    #[serde(default)]
    pub options: Option<ReadableOptionsDto>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateResponse {
    pub readable: bool,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TransformRequest {
    pub html: String,
    #[serde(default)]
    pub options: Option<MarkdownOptionsDto>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TransformResponse {
    pub markdown: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub error: ErrorBody,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ArticleDto {
    title: Option<String>,
    byline: Option<String>,
    dir: Option<String>,
    lang: Option<String>,
    content: String,
    markdown: String,
    text_content: String,
    length: usize,
    excerpt: Option<String>,
    site_name: Option<String>,
    published_time: Option<String>,
    image: Option<String>,
    domain: Option<String>,
    favicon: Option<String>,
}

impl From<lectito::Article> for ArticleDto {
    fn from(article: lectito::Article) -> Self {
        Self {
            title: article.title,
            byline: article.byline,
            dir: article.dir,
            lang: article.lang,
            content: article.content,
            markdown: article.markdown,
            text_content: article.text_content,
            length: article.length,
            excerpt: article.excerpt,
            site_name: article.site_name,
            published_time: article.published_time,
            image: article.image,
            domain: article.domain,
            favicon: article.favicon,
        }
    }
}

#[derive(Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct ReadabilityOptionsDto {
    max_elems_to_parse: Option<Option<usize>>,
    nb_top_candidates: Option<usize>,
    char_threshold: Option<usize>,
    content_selector: Option<Option<String>>,
    site_profiles: Option<Vec<String>>,
    mobile_viewport_width: Option<Option<usize>>,
    classes_to_preserve: Option<Vec<String>>,
    keep_classes: Option<bool>,
    disable_json_ld: Option<bool>,
    link_density_modifier: Option<f32>,
    media_retention: Option<MediaRetentionDto>,
}

impl ReadabilityOptionsDto {
    pub fn into_options(self) -> ReadabilityOptions {
        let mut options = ReadabilityOptions::default();
        if let Some(value) = self.max_elems_to_parse {
            options.max_elems_to_parse = value;
        }
        if let Some(value) = self.nb_top_candidates {
            options.nb_top_candidates = value;
        }
        if let Some(value) = self.char_threshold {
            options.char_threshold = value;
        }
        if let Some(value) = self.content_selector {
            options.content_selector = value;
        }
        if let Some(value) = self.site_profiles {
            options.site_profiles = value;
        }
        if let Some(value) = self.mobile_viewport_width {
            options.mobile_viewport_width = value;
        }
        if let Some(value) = self.classes_to_preserve {
            options.classes_to_preserve = value;
        }
        if let Some(value) = self.keep_classes {
            options.keep_classes = value;
        }
        if let Some(value) = self.disable_json_ld {
            options.disable_json_ld = value;
        }
        if let Some(value) = self.link_density_modifier {
            options.link_density_modifier = value;
        }
        if let Some(value) = self.media_retention {
            options.media_retention = value.into();
        }
        options
    }
}

#[derive(Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct ReadableOptionsDto {
    min_content_length: Option<usize>,
    min_score: Option<f32>,
}

impl ReadableOptionsDto {
    pub fn into_options(self) -> ReadableOptions {
        let mut options = ReadableOptions::default();
        if let Some(value) = self.min_content_length {
            options.min_content_length = value;
        }
        if let Some(value) = self.min_score {
            options.min_score = value;
        }
        options
    }
}

#[derive(Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct MarkdownOptionsDto {
    gfm: Option<bool>,
    footnotes: Option<bool>,
    math: Option<bool>,
    allow_raw_html: Option<bool>,
}

impl From<MarkdownOptionsDto> for MarkdownOptions {
    fn from(val: MarkdownOptionsDto) -> Self {
        let mut options = MarkdownOptions::default();
        if let Some(value) = val.gfm {
            options.gfm = value;
        }
        if let Some(value) = val.footnotes {
            options.footnotes = value;
        }
        if let Some(value) = val.math {
            options.math = value;
        }
        if let Some(value) = val.allow_raw_html {
            options.allow_raw_html = value;
        }
        options
    }
}
