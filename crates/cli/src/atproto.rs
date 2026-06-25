mod embeds;

use anyhow::{Context, Result};
use lectito::escape_html;
use reqwest::Url;
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::Value;

use crate::utils;

pub const DEFAULT_HANDLE_RESOLVER: &str = "https://public.api.bsky.app";
pub const SITE_STANDARD_DOCUMENT: &str = "site.standard.document";
pub const SITE_STANDARD_PUBLICATION: &str = "site.standard.publication";

#[derive(Clone, Debug)]
pub struct AtprotoClient {
    client: Client,
    handle_resolver: String,
}

impl AtprotoClient {
    pub fn new(client: Client) -> Self {
        Self::with_handle_resolver(client, DEFAULT_HANDLE_RESOLVER)
    }

    pub fn with_handle_resolver(client: Client, handle_resolver: impl Into<String>) -> Self {
        Self { client, handle_resolver: handle_resolver.into().trim_end_matches('/').to_string() }
    }

    pub fn resolve_handle(&self, handle: &str) -> Result<String> {
        let mut url = Url::parse(&self.handle_resolver).context("invalid ATProto handle resolver URL")?;
        url.set_path("/xrpc/com.atproto.identity.resolveHandle");
        url.query_pairs_mut().clear().append_pair("handle", handle);

        let body: ResolveHandleResponse = self.get_json(url.as_str())?;
        if !is_valid_did(&body.did) {
            anyhow::bail!("handle {handle} resolved to invalid DID {:?}", body.did);
        }
        Ok(body.did)
    }

    pub fn resolve_did_pds(&self, did: &str) -> Result<String> {
        let did_url = did_document_url(did)?;
        let doc = self.get_json::<DidDocument>(&did_url)?;
        for service in doc.service {
            if service.id != "#atproto_pds" || service.service_type != "AtprotoPersonalDataServer" {
                continue;
            }
            let endpoint =
                Url::parse(&service.service_endpoint).with_context(|| format!("invalid PDS endpoint for {did}"))?;
            if endpoint.scheme() == "https" {
                return Ok(service.service_endpoint.trim_end_matches('/').to_string());
            }
        }
        anyhow::bail!("DID document has no HTTPS ATProto PDS service: {did}");
    }

    pub fn get_record(&self, at_uri: &str) -> Result<ResolvedRecord> {
        let parsed = AtUri::parse(at_uri)?;
        let did = if is_valid_did(&parsed.authority) {
            parsed.authority.clone()
        } else {
            self.resolve_handle(&parsed.authority)?
        };
        let pds = self.resolve_did_pds(&did)?;
        let url = get_record_url(&pds, &did, &parsed.collection, &parsed.rkey)?;
        let record: RepoRecord = self.get_json(url.as_str())?;

        Ok(ResolvedRecord { repo: did, pds, collection: parsed.collection, value: record.value })
    }

    pub fn standard_site_render_metadata(
        &self, record: &ResolvedRecord, source_url: Option<&str>,
    ) -> Result<StandardSiteRenderMetadata> {
        let Some(document) = standard_site_document(record)? else {
            return Ok(StandardSiteRenderMetadata::default());
        };

        let mut warnings = Vec::new();
        let site_name = match self.get_record(&document.site) {
            Ok(publication) if publication.collection == SITE_STANDARD_PUBLICATION => {
                serde_json::from_value::<SiteStandardPublication>(publication.value)
                    .map(|publication| publication.name)
                    .ok()
                    .filter(|name| !name.trim().is_empty())
            }
            Ok(publication) => {
                warnings.push(format!(
                    "site reference resolved to {}, not {SITE_STANDARD_PUBLICATION}",
                    publication.collection
                ));
                None
            }
            Err(error) => {
                warnings.push(format!(
                    "failed to resolve Standard.site publication metadata: {error:#}"
                ));
                None
            }
        };
        let byline = document_byline(&document)
            .or_else(|| source_url.and_then(|url| source_url_author(url, site_name.as_deref())));

        Ok(StandardSiteRenderMetadata { site_name, byline, warnings })
    }

    fn get_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T> {
        let response = self
            .client
            .get(url)
            .send()
            .with_context(|| format!("GET {url} failed"))?
            .error_for_status()
            .with_context(|| format!("GET {url} failed"))?;
        let text = response
            .text()
            .with_context(|| format!("failed to read response body from {url}"))?;
        serde_json::from_str(&text).with_context(|| format!("failed to decode JSON from {url}"))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AtUri {
    pub authority: String,
    pub collection: String,
    pub rkey: String,
}

impl AtUri {
    pub fn parse(raw: &str) -> Result<Self> {
        let Some(rest) = raw.strip_prefix("at://") else {
            anyhow::bail!("invalid AT URI {raw:?}");
        };
        let mut parts = rest.split('/');
        let authority = parts.next().unwrap_or_default();
        let collection = parts.next().unwrap_or_default();
        let rkey = parts.next().unwrap_or_default();
        if authority.is_empty() || collection.is_empty() || rkey.is_empty() || parts.next().is_some() {
            anyhow::bail!("invalid AT URI {raw:?}");
        }

        Ok(Self { authority: authority.to_string(), collection: collection.to_string(), rkey: rkey.to_string() })
    }
}

#[derive(Clone, Debug)]
pub struct ResolvedRecord {
    pub repo: String,
    pub pds: String,
    pub collection: String,
    pub value: Value,
}

#[derive(Debug, Deserialize)]
struct RepoRecord {
    value: Value,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SiteStandardDocument {
    pub site: String,
    pub title: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, rename = "publishedAt")]
    pub published_at: Option<String>,
    #[serde(default, rename = "textContent")]
    pub text_content: Option<String>,
    #[serde(default)]
    pub content: Option<Value>,
    #[serde(default)]
    pub contributors: Vec<SiteStandardContributor>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SiteStandardPublication {
    pub name: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SiteStandardContributor {
    pub did: String,
    #[serde(default, rename = "displayName")]
    pub display_name: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct StandardSiteRenderMetadata {
    pub site_name: Option<String>,
    pub byline: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct StandardSiteRender {
    pub html: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ResolveHandleResponse {
    did: String,
}

#[derive(Debug, Deserialize)]
struct DidDocument {
    service: Vec<DidService>,
}

#[derive(Debug, Deserialize)]
struct DidService {
    id: String,
    #[serde(rename = "type")]
    service_type: String,
    #[serde(rename = "serviceEndpoint")]
    service_endpoint: String,
}

struct RenderedFootnote {
    id: String,
    html: String,
}

struct StandardSiteRenderer<'a> {
    record: &'a ResolvedRecord,
    footnotes: Vec<RenderedFootnote>,
    warnings: Vec<String>,
}

impl<'a> StandardSiteRenderer<'a> {
    fn new(record: &'a ResolvedRecord) -> Self {
        Self { record, footnotes: Vec::new(), warnings: Vec::new() }
    }

    fn render_content_value(&mut self, value: &Value) -> String {
        match value {
            Value::Array(items) => items
                .iter()
                .map(|item| self.render_content_value(item))
                .collect::<String>(),
            Value::Object(fields) => self.render_content_object(fields),
            Value::String(text) => text_content_html(text),
            _ => String::new(),
        }
    }

    fn render_content_object(&mut self, fields: &serde_json::Map<String, Value>) -> String {
        let block_type = string_field(fields, &["$type", "type"]).to_ascii_lowercase();

        if let Some(block) = fields.get("block") {
            return self.render_content_value(block);
        }
        if block_type.contains("content") {
            return self.render_first_nested(fields, &["pages", "items", "blocks", "children", "content"]);
        }
        if block_type.contains("lineardocument") {
            return self.render_first_nested(fields, &["blocks", "children", "content"]);
        }
        if block_type.ends_with(".block.text") || block_type == "text" {
            return self.text_html(&text_field(fields), fields.get("facets"), true);
        }
        if block_type.contains("unorderedlist")
            || block_type.contains("bulletlist")
            || block_type.contains("unordered-list")
        {
            return self.render_list(fields, false);
        }
        if block_type.contains("orderedlist") || block_type.contains("ordered-list") {
            return self.render_list(fields, true);
        }
        if block_type.contains("heading") || block_type.contains("header") {
            return self.render_heading(fields);
        }
        if block_type.contains("code") {
            return render_code(fields);
        }
        if block_type.contains("table") {
            return self.render_table(fields);
        }
        if block_type.contains("math") || block_type.contains("equation") {
            return self.render_math(fields);
        }
        if block_type.contains("blockquote") || block_type.contains("quote") {
            let inner = if has_text(fields) {
                self.text_html(&text_field(fields), fields.get("facets"), true)
            } else {
                self.render_first_nested(fields, &["content", "children", "blocks", "items"])
            };
            return if !inner.trim().is_empty() {
                format!("<blockquote>{inner}</blockquote>")
            } else {
                Default::default()
            };
        }
        if block_type.contains("horizontalrule") || block_type.contains("horizontal-rule") || block_type == "hr" {
            return "<hr>".to_string();
        }
        if block_type.contains("image") {
            return self.render_image(fields);
        }
        if embeds::is_standard_site_post(&block_type) {
            return embeds::render_standard_site_post(fields);
        }
        if embeds::is_bsky_post(&block_type) {
            return embeds::render_bsky_post(fields);
        }
        if embeds::is_website(&block_type) {
            return embeds::render_website(fields, |value| self.blob_url(value));
        }
        if embeds::is_iframe(&block_type) {
            return embeds::render_iframe(fields);
        }
        if embeds::is_button(&block_type) {
            return embeds::render_button(fields);
        }
        if has_text(fields) {
            return self.text_html(&text_field(fields), fields.get("facets"), true);
        }

        let nested = self.render_first_nested(fields, &["content", "children", "pages", "blocks", "items"]);
        if nested.trim().is_empty() && !block_type.is_empty() {
            self.warn(format!("unsupported Standard.site block type {block_type}"));
        }
        nested
    }

    fn render_first_nested(&mut self, fields: &serde_json::Map<String, Value>, names: &[&str]) -> String {
        names
            .iter()
            .find_map(|name| fields.get(*name))
            .map(|value| self.render_content_value(value))
            .unwrap_or_default()
    }

    fn render_list(&mut self, fields: &serde_json::Map<String, Value>, ordered: bool) -> String {
        let Some(items) = first_value(fields, &["children", "items", "content"]).and_then(Value::as_array) else {
            return String::new();
        };

        let tag = if ordered { "ol" } else { "ul" };
        let mut html = format!("<{tag}>");
        for item in items {
            let body = self.render_list_item(item);
            if !body.trim().is_empty() {
                html.push_str(&format!("<li>{body}</li>"));
            }
        }
        html.push_str(&format!("</{tag}>"));
        html
    }

    fn render_list_item(&mut self, value: &Value) -> String {
        let Some(fields) = value.as_object() else {
            return text_content_html(&value.to_string());
        };

        let mut html = String::new();
        if let Some(content) = fields.get("content") {
            html.push_str(&self.render_content_value(content));
        } else if has_text(fields) {
            html.push_str(&self.text_html(&text_field(fields), fields.get("facets"), false));
        }
        for key in ["children", "orderedListChildren", "unorderedListChildren"] {
            if let Some(value) = fields.get(key) {
                html.push_str(&self.render_content_value(value));
            }
        }
        html
    }

    fn render_heading(&mut self, fields: &serde_json::Map<String, Value>) -> String {
        let text = text_field(fields);
        if text.is_empty() {
            return String::new();
        }
        let level = first_value(fields, &["level"])
            .and_then(Value::as_i64)
            .unwrap_or(2)
            .clamp(1, 6);
        let inner = self.text_html(&text, fields.get("facets"), false);
        format!("<h{level}>{inner}</h{level}>")
    }

    fn render_image(&self, fields: &serde_json::Map<String, Value>) -> String {
        let attrs = fields.get("attrs").and_then(Value::as_object).unwrap_or(fields);
        let src = string_field(attrs, &["src", "url"]);
        let src = if let Some(cid) = src.strip_prefix("blob:") {
            first_value(attrs, &["image", "blob"])
                .and_then(|value| self.blob_url(value))
                .or_else(|| self.blob_url(&Value::String(cid.to_string())))
        } else if src.is_empty() {
            first_value(attrs, &["image", "blob"]).and_then(|value| self.blob_url(value))
        } else {
            Some(src)
        };
        let Some(src) = src.filter(|src| !src.is_empty()) else {
            return String::new();
        };
        let alt = string_field(attrs, &["alt"]);
        let caption = string_field(attrs, &["caption", "credit"]);
        let mut html = format!(
            "<figure><img src=\"{}\" alt=\"{}\">",
            escape_html(&src),
            escape_html(&alt)
        );
        if !caption.is_empty() {
            html.push_str(&format!("<figcaption>{}</figcaption>", escape_html(&caption)));
        }
        html.push_str("</figure>");
        html
    }

    fn render_table(&mut self, fields: &serde_json::Map<String, Value>) -> String {
        let Some(rows) = first_value(fields, &["rows", "children", "items", "content"]).and_then(Value::as_array)
        else {
            self.warn("table block did not contain rows".to_string());
            return String::new();
        };

        let mut html = String::from("<table><tbody>");
        for row in rows {
            let Some(cells) = table_row_cells(row) else {
                continue;
            };
            html.push_str("<tr>");
            let row_header = row
                .as_object()
                .map(|fields| truthy_field(fields, &["header", "isHeader", "heading"]))
                .unwrap_or(false);
            for cell in cells {
                let cell_header = cell
                    .as_object()
                    .map(|fields| truthy_field(fields, &["header", "isHeader", "heading"]))
                    .unwrap_or(false);
                let tag = if row_header || cell_header { "th" } else { "td" };
                let body = self.render_table_cell(cell);
                html.push_str(&format!("<{tag}>{body}</{tag}>"));
            }
            html.push_str("</tr>");
        }
        html.push_str("</tbody></table>");
        html
    }

    fn render_table_cell(&mut self, value: &Value) -> String {
        if let Some(fields) = value.as_object() {
            if let Some(content) = first_value(fields, &["content", "children", "blocks", "items"]) {
                if let Some(content_fields) = content.as_object()
                    && has_text(content_fields)
                {
                    return self.text_html(&text_field(content_fields), content_fields.get("facets"), false);
                }
                return self.render_content_value(content);
            }
            if has_text(fields) {
                return self.text_html(&text_field(fields), fields.get("facets"), false);
            }
        }
        match value {
            Value::String(text) => escape_html(text),
            Value::Number(_) | Value::Bool(_) => escape_html(&value.to_string()),
            _ => String::new(),
        }
    }

    fn render_math(&mut self, fields: &serde_json::Map<String, Value>) -> String {
        let latex = string_field(fields, &["latex", "tex", "formula", "expression", "plaintext", "text"]);
        if latex.is_empty() {
            self.warn("math block did not contain latex text".to_string());
            return String::new();
        }
        let display = truthy_field(fields, &["display", "displayMode", "block"])
            || string_field(fields, &["mode"]).eq_ignore_ascii_case("display");
        if display {
            format!(
                "<div class=\"math display\"><span data-latex=\"{}\" display=\"block\"></span></div>",
                escape_html(&latex)
            )
        } else {
            format!("<span class=\"math\" data-latex=\"{}\"></span>", escape_html(&latex))
        }
    }

    fn render_footnotes(&self) -> String {
        if self.footnotes.is_empty() {
            return String::new();
        }
        let mut html = String::from("<section class=\"footnotes\"><ol>");
        for footnote in &self.footnotes {
            html.push_str(&format!(
                "<li id=\"fn-{}\">{} <a href=\"#fnref-{}\">Back</a></li>",
                escape_html(&footnote.id),
                footnote.html,
                escape_html(&footnote.id)
            ));
        }
        html.push_str("</ol></section>");
        html
    }

    fn render_facet_text(&mut self, text: &str, features: &[RichTextFeature]) -> String {
        let mut html = escape_html_with_breaks(text);
        for feature in features.iter().rev() {
            html = match feature {
                RichTextFeature::Link(uri) => {
                    format!("<a href=\"{}\">{html}</a>", escape_html(uri))
                }
                RichTextFeature::Bold => format!("<strong>{html}</strong>"),
                RichTextFeature::Italic => format!("<em>{html}</em>"),
                RichTextFeature::Code => format!("<code>{html}</code>"),
                RichTextFeature::Underline => format!("<u>{html}</u>"),
                RichTextFeature::Strikethrough => format!("<s>{html}</s>"),
                RichTextFeature::Id(id) => format!("<span id=\"{}\">{html}</span>", escape_html(id)),
                RichTextFeature::Footnote { id, text, facets } => {
                    let content = self.rich_text_html(text, Some(facets));
                    self.push_footnote(id, content);
                    format!(
                        "{html}<sup id=\"fnref-{0}\"><a href=\"#fn-{0}\">{0}</a></sup>",
                        escape_html(id)
                    )
                }
            };
        }
        html
    }

    fn push_footnote(&mut self, id: &str, html: String) {
        if id.trim().is_empty() || html.trim().is_empty() || self.footnotes.iter().any(|footnote| footnote.id == id) {
            return;
        }
        self.footnotes.push(RenderedFootnote { id: id.to_string(), html });
    }

    fn warn(&mut self, warning: String) {
        if !self.warnings.iter().any(|existing| existing == &warning) {
            self.warnings.push(warning);
        }
    }

    fn blob_url(&self, value: &Value) -> Option<String> {
        let cid = blob_cid(value)?;
        let mut url = Url::parse(&self.record.pds).ok()?;
        url.set_path("/xrpc/com.atproto.sync.getBlob");
        url.query_pairs_mut()
            .clear()
            .append_pair("did", &self.record.repo)
            .append_pair("cid", &cid);
        Some(url.to_string())
    }

    fn text_html(&mut self, text: &str, facets: Option<&Value>, paragraph: bool) -> String {
        let inner = self.rich_text_html(text, facets);
        if inner.trim().is_empty() {
            return String::new();
        }
        if paragraph { format!("<p>{inner}</p>") } else { inner }
    }

    fn rich_text_html(&mut self, text: &str, facets: Option<&Value>) -> String {
        let mut ranges = facets
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(RichTextRange::from_value)
            .collect::<Vec<_>>();
        ranges.sort_by_key(|range| (range.start, range.end));

        let bytes = text.as_bytes();
        let mut cursor = 0;
        let mut html = String::new();
        for range in ranges {
            if range.start < cursor
                || range.end > bytes.len()
                || !text.is_char_boundary(range.start)
                || !text.is_char_boundary(range.end)
            {
                continue;
            }
            html.push_str(&escape_html_with_breaks(&text[cursor..range.start]));
            html.push_str(&self.render_facet_text(&text[range.start..range.end], &range.features));
            cursor = range.end;
        }
        html.push_str(&escape_html_with_breaks(&text[cursor..]));
        html
    }
}

#[derive(Clone, Debug)]
enum RichTextFeature {
    Link(String),
    Bold,
    Italic,
    Code,
    Underline,
    Strikethrough,
    Id(String),
    Footnote { id: String, text: String, facets: Value },
}

impl RichTextFeature {
    // TODO: could this be from/into &/or a de/serialize implementation?
    fn from_value(value: &Value) -> Option<Self> {
        let fields = value.as_object()?;
        let feature_type = string_field(fields, &["$type", "type"]).to_ascii_lowercase();
        if feature_type.contains("link") || feature_type.contains("mention") {
            let uri = string_field(fields, &["uri", "href", "atURI"]);
            return (!uri.is_empty()).then_some(Self::Link(uri));
        }
        if feature_type.contains("bold") {
            return Some(Self::Bold);
        }
        if feature_type.contains("italic") {
            return Some(Self::Italic);
        }
        if feature_type.contains("code") {
            return Some(Self::Code);
        }
        if feature_type.contains("underline") {
            return Some(Self::Underline);
        }
        if feature_type.contains("strikethrough") {
            return Some(Self::Strikethrough);
        }
        if feature_type.ends_with("#id") || feature_type.contains(".id") {
            let id = string_field(fields, &["id"]);
            return (!id.is_empty()).then_some(Self::Id(id));
        }
        if feature_type.contains("footnote") {
            let id = string_field(fields, &["footnoteId"]);
            let text = string_field(fields, &["contentPlaintext"]);
            let facets = fields
                .get("contentFacets")
                .cloned()
                .unwrap_or_else(|| Value::Array(Vec::new()));
            return (!id.is_empty() && !text.is_empty()).then_some(Self::Footnote { id, text, facets });
        }
        None
    }
}

#[derive(Clone, Debug)]
struct RichTextRange {
    start: usize,
    end: usize,
    features: Vec<RichTextFeature>,
}

impl RichTextRange {
    fn from_value(value: &Value) -> Option<Self> {
        let fields = value.as_object()?;
        let index = fields.get("index")?.as_object()?;
        let start = first_value(index, &["byteStart"])?.as_u64()? as usize;
        let end = first_value(index, &["byteEnd"])?.as_u64()? as usize;
        if start >= end {
            return None;
        }
        let features = fields
            .get("features")?
            .as_array()?
            .iter()
            .filter_map(RichTextFeature::from_value)
            .collect::<Vec<_>>();
        (!features.is_empty()).then_some(Self { start, end, features })
    }
}

pub fn standard_site_link(html: &str) -> Option<String> {
    let document = scraper::Html::parse_document(html);
    let document_selector =
        scraper::Selector::parse("link[rel='site.standard.document'][href]").expect("valid selector");
    let publication_selector =
        scraper::Selector::parse("link[rel='site.standard.publication'][href]").expect("valid selector");

    document
        .select(&document_selector)
        .next()
        .and_then(|node| node.value().attr("href"))
        .or_else(|| {
            document
                .select(&publication_selector)
                .next()
                .and_then(|node| node.value().attr("href"))
        })
        .map(str::to_string)
}

pub fn standard_site_document_render(
    record: &ResolvedRecord, source_url: Option<&str>, metadata: &StandardSiteRenderMetadata,
) -> Result<Option<StandardSiteRender>> {
    let Some(document) = standard_site_document(record)? else {
        return Ok(None);
    };
    let mut renderer = StandardSiteRenderer::new(record);
    let body = document
        .content
        .as_ref()
        .map(|value| renderer.render_content_value(value))
        .filter(|html| !html.trim().is_empty())
        .or_else(|| document.text_content.as_deref().map(text_content_html))
        .filter(|html| !html.trim().is_empty());
    let Some(body) = body else {
        return Ok(None);
    };

    let mut html = String::from("<!doctype html><html><head><meta charset=\"utf-8\">");
    html.push_str(&format!("<title>{}</title>", escape_html(&document.title)));
    html.push_str(&format!(
        "<meta property=\"og:title\" content=\"{}\">",
        escape_html(&document.title)
    ));
    if let Some(site_name) = metadata.site_name.as_deref().filter(|value| !value.trim().is_empty()) {
        html.push_str(&format!(
            "<meta property=\"og:site_name\" content=\"{}\">",
            escape_html(site_name)
        ));
    }
    if let Some(byline) = metadata.byline.as_deref().filter(|value| !value.trim().is_empty()) {
        html.push_str(&format!("<meta name=\"author\" content=\"{}\">", escape_html(byline)));
    }
    if let Some(description) = document.description.as_deref().filter(|value| !value.trim().is_empty()) {
        html.push_str(&format!(
            "<meta name=\"description\" content=\"{}\">",
            escape_html(description)
        ));
    }
    if let Some(source_url) = source_url.filter(|value| !value.trim().is_empty()) {
        html.push_str(&format!(
            "<link rel=\"canonical\" href=\"{}\">",
            escape_html(source_url)
        ));
    }
    html.push_str("</head><body><article>");
    html.push_str(&format!("<h1>{}</h1>", escape_html(&document.title)));
    if let Some(byline) = metadata.byline.as_deref().filter(|value| !value.trim().is_empty()) {
        html.push_str(&format!("<p class=\"byline\">{}</p>", escape_html(byline)));
    }
    if let Some(description) = document.description.as_deref().filter(|value| !value.trim().is_empty()) {
        html.push_str(&format!("<p><em>{}</em></p>", escape_html(description)));
    }
    if let Some(published_at) = document
        .published_at
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        html.push_str(&format!(
            "<p><time datetime=\"{}\">{}</time></p>",
            escape_html(published_at),
            escape_html(published_at)
        ));
    }
    html.push_str(&body);
    html.push_str(&renderer.render_footnotes());
    html.push_str("</article></body></html>");
    let mut warnings = metadata.warnings.clone();
    warnings.extend(renderer.warnings);
    Ok(Some(StandardSiteRender { html, warnings }))
}

pub fn standard_site_document(record: &ResolvedRecord) -> Result<Option<SiteStandardDocument>> {
    if record.collection != SITE_STANDARD_DOCUMENT {
        return Ok(None);
    }
    serde_json::from_value(record.value.clone())
        .map(Some)
        .context("invalid site.standard.document record")
}

fn render_code(fields: &serde_json::Map<String, Value>) -> String {
    let code = string_field(fields, &["code", "plaintext", "text"]);
    if code.is_empty() {
        return String::new();
    }
    let language = string_field(fields, &["language", "lang"]);
    if language.is_empty() {
        format!("<pre><code>{}</code></pre>", escape_html(&code))
    } else {
        format!(
            "<pre><code class=\"language-{}\">{}</code></pre>",
            escape_html(&language),
            escape_html(&code)
        )
    }
}

fn document_byline(document: &SiteStandardDocument) -> Option<String> {
    let contributors = document
        .contributors
        .iter()
        .filter_map(|contributor| {
            contributor
                .display_name
                .as_deref()
                .filter(|name| !name.trim().is_empty())
                .or_else(|| (!contributor.did.trim().is_empty()).then_some(contributor.did.as_str()))
        })
        .map(str::trim)
        .collect::<Vec<_>>();
    if !contributors.is_empty() {
        return Some(contributors.join(", "));
    }

    document
        .author
        .as_deref()
        .map(str::trim)
        .filter(|author| !author.is_empty())
        .map(str::to_string)
}

fn source_url_author(source_url: &str, site_name: Option<&str>) -> Option<String> {
    let url = Url::parse(source_url).ok()?;
    let host = url.host_str()?.trim_start_matches("www.");
    let parts = host.split('.').collect::<Vec<_>>();
    if parts.len() < 3 {
        return None;
    }
    let base = parts[parts.len() - 2..].join(".");
    match base.as_str() {
        "offprint.app" => site_name
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .or_else(|| Some(parts[0].to_string())),
        "leaflet.pub" | "pckt.blog" => Some(parts[0].to_string()),
        _ => None,
    }
}

fn text_content_html(text: &str) -> String {
    text.replace("\r\n", "\n")
        .split("\n\n")
        .filter_map(|part| {
            let part = part.trim();
            (!part.is_empty()).then(|| format!("<p>{}</p>", escape_html(part)))
        })
        .collect::<String>()
}

fn escape_html_with_breaks(text: &str) -> String {
    escape_html(text).replace('\n', "<br>")
}

fn blob_cid(value: &Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        return (!text.trim().is_empty()).then(|| text.trim().to_string());
    }
    let fields = value.as_object()?;
    let cid = string_field(fields, &["cid", "$link"]);
    if !cid.is_empty() {
        return Some(cid);
    }
    fields
        .get("ref")
        .and_then(|ref_value| ref_value.as_object())
        .and_then(|ref_fields| blob_cid(&Value::Object(ref_fields.clone())))
}

fn has_text(fields: &serde_json::Map<String, Value>) -> bool {
    !text_field(fields).is_empty()
}

fn text_field(fields: &serde_json::Map<String, Value>) -> String {
    raw_string_field(fields, &["plaintext", "text", "body", "title"])
}

fn string_field(fields: &serde_json::Map<String, Value>, names: &[&str]) -> String {
    raw_string_field(fields, names).trim().to_string()
}

fn raw_string_field(fields: &serde_json::Map<String, Value>, names: &[&str]) -> String {
    first_value(fields, names)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn first_value<'a>(fields: &'a serde_json::Map<String, Value>, names: &[&str]) -> Option<&'a Value> {
    names.iter().find_map(|name| fields.get(*name))
}

fn truthy_field(fields: &serde_json::Map<String, Value>, names: &[&str]) -> bool {
    first_value(fields, names).is_some_and(|value| {
        value.as_bool().unwrap_or_else(|| {
            value
                .as_str()
                .is_some_and(|text| matches!(text.to_ascii_lowercase().as_str(), "true" | "yes" | "1" | "display"))
        })
    })
}

fn table_row_cells(value: &Value) -> Option<&Vec<Value>> {
    if let Some(cells) = value.as_array() {
        return Some(cells);
    }
    let fields = value.as_object()?;
    first_value(fields, &["cells", "children", "items", "content"]).and_then(Value::as_array)
}

fn did_document_url(did: &str) -> Result<String> {
    if let Some(id) = did.strip_prefix("did:plc:") {
        if id.is_empty() {
            anyhow::bail!("invalid did:plc {did:?}");
        }
        return Ok(format!("https://plc.directory/{did}"));
    }

    if let Some(id) = did.strip_prefix("did:web:") {
        let mut parts = id
            .split(':')
            .map(utils::decode_percentage)
            .collect::<Result<Vec<_>>>()
            .with_context(|| format!("invalid did:web {did:?}"))?;
        if parts.first().is_none_or(|host| host.is_empty()) {
            anyhow::bail!("invalid did:web {did:?}");
        }
        let host = parts.remove(0);
        let path = if parts.is_empty() { ".well-known".to_string() } else { parts.join("/") };
        return Ok(format!("https://{host}/{path}/did.json"));
    }

    anyhow::bail!("unsupported DID method: {did}");
}

fn get_record_url(pds: &str, repo: &str, collection: &str, rkey: &str) -> Result<Url> {
    let mut url = Url::parse(pds).with_context(|| format!("invalid PDS endpoint {pds:?}"))?;
    url.set_path("/xrpc/com.atproto.repo.getRecord");
    url.query_pairs_mut()
        .clear()
        .append_pair("repo", repo)
        .append_pair("collection", collection)
        .append_pair("rkey", rkey);
    Ok(url)
}

fn is_valid_did(value: &str) -> bool {
    let Some((method, id)) = value.strip_prefix("did:").and_then(|rest| rest.split_once(':')) else {
        return false;
    };
    !method.is_empty()
        && method
            .chars()
            .all(|char| char.is_ascii_lowercase() || char.is_ascii_digit())
        && !id.is_empty()
        && id
            .chars()
            .all(|char| char.is_ascii_alphanumeric() || matches!(char, '.' | '_' | ':' | '%' | '-'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_at_uri() {
        let uri = AtUri::parse("at://did:plc:abc/site.standard.document/3mk4xkaxobc2p").unwrap();

        assert_eq!(uri.authority, "did:plc:abc");
        assert_eq!(uri.collection, SITE_STANDARD_DOCUMENT);
        assert_eq!(uri.rkey, "3mk4xkaxobc2p");
    }

    #[test]
    fn rejects_invalid_at_uri() {
        assert!(AtUri::parse("https://example.com").is_err());
        assert!(AtUri::parse("at://did:plc:abc/site.standard.document").is_err());
        assert!(AtUri::parse("at://did:plc:abc/site.standard.document/rkey/extra").is_err());
    }

    #[test]
    fn maps_did_document_urls() {
        assert_eq!(
            did_document_url("did:plc:abc123").unwrap(),
            "https://plc.directory/did:plc:abc123"
        );
        assert_eq!(
            did_document_url("did:web:example.com").unwrap(),
            "https://example.com/.well-known/did.json"
        );
        assert_eq!(
            did_document_url("did:web:example.com:users:alice").unwrap(),
            "https://example.com/users/alice/did.json"
        );
        assert_eq!(
            did_document_url("did:web:example.com:users%3Aalice").unwrap(),
            "https://example.com/users:alice/did.json"
        );
    }

    #[test]
    fn builds_xrpc_urls() {
        let url = get_record_url("https://pds.example", "did:plc:abc", SITE_STANDARD_DOCUMENT, "rkey").unwrap();

        assert_eq!(
            url.as_str(),
            "https://pds.example/xrpc/com.atproto.repo.getRecord?repo=did%3Aplc%3Aabc&collection=site.standard.document&rkey=rkey"
        );
    }

    #[test]
    fn finds_standard_site_document_before_publication() {
        let html = r#"
            <html>
              <head>
                <link rel="site.standard.publication" href="at://did:plc:abc/site.standard.publication/main">
                <link rel="site.standard.document" href="at://did:plc:abc/site.standard.document/rkey">
              </head>
            </html>
        "#;

        assert_eq!(
            standard_site_link(html).as_deref(),
            Some("at://did:plc:abc/site.standard.document/rkey")
        );
    }

    #[test]
    fn renders_rich_text_facets_in_standard_site_content() {
        let record = resolved_document(json!({
            "site": "at://did:plc:abc/site.standard.publication/main",
            "title": "Facets",
            "publishedAt": "2026-06-24T00:00:00Z",
            "content": {
                "$type": "pub.leaflet.pages.linearDocument",
                "blocks": [
                    {
                        "block": {
                            "$type": "pub.leaflet.blocks.text",
                            "plaintext": "Read this now",
                            "facets": [
                                {
                                    "index": { "byteStart": 0, "byteEnd": 4 },
                                    "features": [{ "$type": "pub.leaflet.richtext.facet#bold" }]
                                },
                                {
                                    "index": { "byteStart": 5, "byteEnd": 9 },
                                    "features": [{
                                        "$type": "pub.leaflet.richtext.facet#link",
                                        "uri": "https://example.com"
                                    }]
                                }
                            ]
                        }
                    }
                ]
            }
        }));

        let html = rendered_html(&record);

        assert!(html.contains("<p><strong>Read</strong> <a href=\"https://example.com\">this</a> now</p>"));
    }

    #[test]
    fn resolves_blob_images_to_pds_blob_urls() {
        let record = resolved_document(json!({
            "site": "at://did:plc:abc/site.standard.publication/main",
            "title": "Image",
            "publishedAt": "2026-06-24T00:00:00Z",
            "content": {
                "$type": "pub.leaflet.pages.linearDocument",
                "blocks": [
                    {
                        "block": {
                            "$type": "pub.leaflet.blocks.image",
                            "alt": "A diagram",
                            "image": {
                                "$type": "blob",
                                "ref": { "$link": "bafkreidemo" },
                                "mimeType": "image/png",
                                "size": 123
                            },
                            "aspectRatio": { "width": 4, "height": 3 }
                        }
                    }
                ]
            }
        }));

        let html = rendered_html(&record);

        assert!(html.contains(
            "<img src=\"https://pds.example/xrpc/com.atproto.sync.getBlob?did=did%3Aplc%3Aabc&amp;cid=bafkreidemo\" alt=\"A diagram\">"
        ));
    }

    #[test]
    fn preserves_image_alt_text_and_caption() {
        let record = resolved_document(json!({
            "site": "at://did:plc:abc/site.standard.publication/main",
            "title": "Captioned image",
            "publishedAt": "2026-06-24T00:00:00Z",
            "content": {
                "$type": "pub.leaflet.pages.linearDocument",
                "blocks": [{
                    "block": {
                        "$type": "pub.leaflet.blocks.image",
                        "src": "https://example.com/forest.jpg",
                        "alt": "Sunlight through trees",
                        "caption": "Northern New Mexico forest"
                    }
                }]
            }
        }));

        let html = rendered_html(&record);

        assert!(html.contains(
            "<figure><img src=\"https://example.com/forest.jpg\" alt=\"Sunlight through trees\"><figcaption>Northern New Mexico forest</figcaption></figure>"
        ));
    }

    #[test]
    fn renders_tables_from_publisher_blocks() {
        let record = resolved_document(json!({
            "site": "at://did:plc:abc/site.standard.publication/main",
            "title": "Table",
            "publishedAt": "2026-06-24T00:00:00Z",
            "content": {
                "$type": "pub.leaflet.pages.linearDocument",
                "blocks": [{
                    "block": {
                        "$type": "pub.leaflet.blocks.table",
                        "rows": [
                            { "header": true, "cells": [
                                { "text": "Name" },
                                { "text": "Value" }
                            ]},
                            { "cells": [
                                { "text": "Leaflet" },
                                {
                                    "content": {
                                        "$type": "pub.leaflet.blocks.text",
                                        "plaintext": "supports links",
                                        "facets": [{
                                            "index": { "byteStart": 9, "byteEnd": 14 },
                                            "features": [{
                                                "$type": "pub.leaflet.richtext.facet#link",
                                                "uri": "https://standard.site/"
                                            }]
                                        }]
                                    }
                                }
                            ]}
                        ]
                    }
                }]
            }
        }));

        let html = rendered_html(&record);

        assert!(html.contains("<table><tbody><tr><th>Name</th><th>Value</th></tr>"));
        assert!(
            html.contains("<tr><td>Leaflet</td><td>supports <a href=\"https://standard.site/\">links</a></td></tr>")
        );
    }

    #[test]
    fn renders_math_blocks_as_latex_markdown_source() {
        let record = resolved_document(json!({
            "site": "at://did:plc:abc/site.standard.publication/main",
            "title": "Math",
            "publishedAt": "2026-06-24T00:00:00Z",
            "content": {
                "$type": "pub.leaflet.pages.linearDocument",
                "blocks": [{
                    "block": {
                        "$type": "pub.leaflet.blocks.math",
                        "latex": "E = mc^2",
                        "display": true
                    }
                }]
            }
        }));

        let html = rendered_html(&record);
        let markdown = lectito::html_to_markdown(&html);

        assert!(html.contains("<span data-latex=\"E = mc^2\" display=\"block\"></span>"));
        assert!(markdown.contains("$$\nE = mc^2\n$$"), "{markdown}");
    }

    #[test]
    fn reports_rendering_warnings_for_unknown_blocks() {
        let record = resolved_document(json!({
            "site": "at://did:plc:abc/site.standard.publication/main",
            "title": "Unknown",
            "publishedAt": "2026-06-24T00:00:00Z",
            "content": {
                "$type": "pub.leaflet.pages.linearDocument",
                "blocks": [{
                    "block": { "$type": "pub.leaflet.blocks.customWidget" }
                }]
            },
            "textContent": "Fallback"
        }));

        let render = standard_site_document_render(&record, None, &StandardSiteRenderMetadata::default())
            .unwrap()
            .unwrap();

        assert!(
            render
                .warnings
                .contains(&"unsupported Standard.site block type pub.leaflet.blocks.customwidget".to_string())
        );
    }

    #[test]
    fn renders_offprint_text_blocks_without_warnings() {
        let record = resolved_document(json!({
            "site": "at://did:plc:abc/site.standard.publication/main",
            "title": "Offprint",
            "publishedAt": "2026-06-24T00:00:00Z",
            "content": {
                "$type": "app.offprint.content",
                "items": [
                    {
                        "$type": "app.offprint.block.text",
                        "plaintext": "Worth building",
                        "facets": [{
                            "index": { "byteStart": 0, "byteEnd": 5 },
                            "features": [{ "$type": "app.offprint.richtext.facet#italic" }]
                        }]
                    },
                    { "$type": "app.offprint.block.text", "plaintext": "" }
                ]
            }
        }));

        let render = standard_site_document_render(&record, None, &StandardSiteRenderMetadata::default())
            .unwrap()
            .unwrap();

        assert!(render.html.contains("<p><em>Worth</em> building</p>"));
        assert!(render.warnings.is_empty(), "{:?}", render.warnings);
    }

    #[test]
    fn renders_frozen_standard_site_record_examples() {
        let examples = [
            (
                "leaflet",
                include_str!("../../fixtures/atproto/leaflet.json"),
                "did:plc:4vjd3fe2cgzq5d24j4f3zvar",
                "https://hydnum.us-west.host.bsky.network",
                "Skyreader update",
            ),
            (
                "pckt",
                include_str!("../../fixtures/atproto/pckt.json"),
                "did:plc:lqmen5vsre5oabzmepfv3r7e",
                "https://shimeji.us-east.host.bsky.network",
                "Hello, Forest",
            ),
            (
                "offprint",
                include_str!("../../fixtures/atproto/offprint.json"),
                "did:plc:4hodhjl2kposuchzvpiviwps",
                "https://gomphus.us-west.host.bsky.network",
                "Agentic Software Development",
            ),
        ];

        for (name, json, repo, pds, expected) in examples {
            let record = ResolvedRecord {
                repo: repo.to_string(),
                pds: pds.to_string(),
                collection: SITE_STANDARD_DOCUMENT.to_string(),
                value: serde_json::from_str(json).expect("fixture record"),
            };
            let render = standard_site_document_render(&record, None, &StandardSiteRenderMetadata::default())
                .unwrap()
                .unwrap();
            let markdown = lectito::html_to_markdown(&render.html);

            assert!(render.html.contains(expected), "{name} html:\n{}", render.html);
            assert!(markdown.contains(expected), "{name} markdown:\n{markdown}");
            if name == "pckt" {
                assert!(
                    render
                        .html
                        .contains("https://shimeji.us-east.host.bsky.network/xrpc/com.atproto.sync.getBlob"),
                    "{name} html:\n{}",
                    render.html
                );
            }
        }
    }

    #[test]
    fn renders_footnotes_from_rich_text_facets() {
        let record = resolved_document(json!({
            "site": "at://did:plc:abc/site.standard.publication/main",
            "title": "Footnotes",
            "publishedAt": "2026-06-24T00:00:00Z",
            "content": {
                "$type": "pub.leaflet.pages.linearDocument",
                "blocks": [
                    {
                        "block": {
                            "$type": "pub.leaflet.blocks.text",
                            "plaintext": "Claim",
                            "facets": [{
                                "index": { "byteStart": 0, "byteEnd": 5 },
                                "features": [{
                                    "$type": "pub.leaflet.richtext.facet#footnote",
                                    "footnoteId": "1",
                                    "contentPlaintext": "Source link",
                                    "contentFacets": [{
                                        "index": { "byteStart": 7, "byteEnd": 11 },
                                        "features": [{
                                            "$type": "pub.leaflet.richtext.facet#link",
                                            "uri": "https://example.com/source"
                                        }]
                                    }]
                                }]
                            }]
                        }
                    }
                ]
            }
        }));

        let html = rendered_html(&record);

        assert!(html.contains("<p>Claim<sup id=\"fnref-1\"><a href=\"#fn-1\">1</a></sup></p>"));
        assert!(html.contains(
            "<section class=\"footnotes\"><ol><li id=\"fn-1\">Source <a href=\"https://example.com/source\">link</a> <a href=\"#fnref-1\">Back</a></li></ol></section>"
        ));
    }

    #[test]
    fn renders_embedded_standard_site_posts() {
        let record = resolved_document(json!({
            "site": "at://did:plc:abc/site.standard.publication/main",
            "title": "Collection",
            "publishedAt": "2026-06-24T00:00:00Z",
            "content": {
                "$type": "pub.leaflet.pages.linearDocument",
                "blocks": [{
                    "block": {
                        "$type": "pub.leaflet.blocks.standardSitePost",
                        "uri": "at://did:plc:def/site.standard.document/3moxvif7ejk2i",
                        "cid": "bafyreidemo"
                    }
                }]
            }
        }));

        let html = rendered_html(&record);

        assert!(html.contains(
            "<blockquote><p>Embedded Standard.site post: <a href=\"at://did:plc:def/site.standard.document/3moxvif7ejk2i\">at://did:plc:def/site.standard.document/3moxvif7ejk2i</a></p></blockquote>"
        ));
    }

    #[test]
    fn renders_bluesky_post_embeds_as_links() {
        let record = resolved_document(json!({
            "site": "at://did:plc:abc/site.standard.publication/main",
            "title": "Bluesky",
            "publishedAt": "2026-06-24T00:00:00Z",
            "content": {
                "$type": "pub.leaflet.pages.linearDocument",
                "blocks": [{
                    "block": {
                        "$type": "pub.leaflet.blocks.bskyPost",
                        "postRef": {
                            "cid": "bafyreih2gyc6dcqmuimiihfvlkesedwkgcpa7n62wu4mamdl23kh7vheqi",
                            "uri": "at://did:plc:f4os2wz5fjl56xpwcvtnqu7m/app.bsky.feed.post/3moluu6nxfs2q"
                        },
                        "clientHost": "bsky.app"
                    }
                }]
            }
        }));

        let html = rendered_html(&record);

        assert!(html.contains(
            "<blockquote><p>Embedded Bluesky post: <a href=\"https://bsky.app/profile/did:plc:f4os2wz5fjl56xpwcvtnqu7m/post/3moluu6nxfs2q\">https://bsky.app/profile/did:plc:f4os2wz5fjl56xpwcvtnqu7m/post/3moluu6nxfs2q</a></p></blockquote>"
        ));
    }

    #[test]
    fn renders_web_bookmarks_embeds_and_buttons() {
        let record = resolved_document(json!({
            "site": "at://did:plc:abc/site.standard.publication/main",
            "title": "Web",
            "publishedAt": "2026-06-24T00:00:00Z",
            "content": {
                "$type": "pub.leaflet.pages.linearDocument",
                "blocks": [
                    {
                        "block": {
                            "$type": "pub.leaflet.blocks.website",
                            "src": "https://standard.site/docs/lexicons/document/",
                            "title": "Document Lexicon",
                            "description": "Schema reference",
                            "previewImage": {
                                "$type": "blob",
                                "ref": { "$link": "bafkreiwebsite" },
                                "mimeType": "image/png",
                                "size": 123
                            }
                        }
                    },
                    {
                        "block": {
                            "$type": "pub.leaflet.blocks.iframe",
                            "url": "https://example.com/embed"
                        }
                    },
                    {
                        "block": {
                            "$type": "pub.leaflet.blocks.button",
                            "url": "https://leaflet.pub/checkout/pro",
                            "text": "Get Leaflet Pro"
                        }
                    }
                ]
            }
        }));

        let html = rendered_html(&record);

        assert!(html.contains(
            "<p><a href=\"https://standard.site/docs/lexicons/document/\">Document Lexicon</a></p><p>Schema reference</p><figure><img src=\"https://pds.example/xrpc/com.atproto.sync.getBlob?did=did%3Aplc%3Aabc&amp;cid=bafkreiwebsite\" alt=\"\"></figure>"
        ));
        assert!(html.contains(
            "<iframe src=\"https://example.com/embed\" loading=\"lazy\" referrerpolicy=\"no-referrer-when-downgrade\"></iframe>"
        ));
        assert!(html.contains("<p><a href=\"https://leaflet.pub/checkout/pro\">Get Leaflet Pro</a></p>"));
    }

    fn rendered_html(record: &ResolvedRecord) -> String {
        standard_site_document_render(record, None, &StandardSiteRenderMetadata::default())
            .unwrap()
            .unwrap()
            .html
    }

    fn resolved_document(value: Value) -> ResolvedRecord {
        ResolvedRecord {
            repo: "did:plc:abc".to_string(),
            pds: "https://pds.example".to_string(),
            collection: SITE_STANDARD_DOCUMENT.to_string(),
            value,
        }
    }
}
