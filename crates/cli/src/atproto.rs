use anyhow::{Context, Result};
use lectito::html::escape_html;
use reqwest::Url;
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::Value;

pub const DEFAULT_HANDLE_RESOLVER: &str = "https://public.api.bsky.app";
pub const SITE_STANDARD_DOCUMENT: &str = "site.standard.document";
pub const SITE_STANDARD_PUBLICATION: &str = "site.standard.publication";

#[derive(Clone, Debug)]
pub struct AtprotoClient {
    client: Client,
    handle_resolver: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AtUri {
    pub authority: String,
    pub collection: String,
    pub rkey: String,
}

#[derive(Clone, Debug)]
pub struct ResolvedRecord {
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

        Ok(ResolvedRecord { collection: parsed.collection, value: record.value })
    }

    pub fn standard_site_render_metadata(
        &self, record: &ResolvedRecord, source_url: Option<&str>,
    ) -> Result<StandardSiteRenderMetadata> {
        let Some(document) = standard_site_document(record)? else {
            return Ok(StandardSiteRenderMetadata::default());
        };

        let site_name = self
            .get_record(&document.site)
            .ok()
            .filter(|publication| publication.collection == SITE_STANDARD_PUBLICATION)
            .and_then(|publication| serde_json::from_value::<SiteStandardPublication>(publication.value).ok())
            .map(|publication| publication.name)
            .filter(|name| !name.trim().is_empty());
        let byline = document_byline(&document)
            .or_else(|| source_url.and_then(|url| source_url_author(url, site_name.as_deref())));

        Ok(StandardSiteRenderMetadata { site_name, byline })
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

pub fn standard_site_document_html(
    record: &ResolvedRecord, source_url: Option<&str>, metadata: &StandardSiteRenderMetadata,
) -> Result<Option<String>> {
    let Some(document) = standard_site_document(record)? else {
        return Ok(None);
    };
    let body = document
        .content
        .as_ref()
        .map(render_content_value)
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
    html.push_str("</article></body></html>");
    Ok(Some(html))
}

pub fn standard_site_document(record: &ResolvedRecord) -> Result<Option<SiteStandardDocument>> {
    if record.collection != SITE_STANDARD_DOCUMENT {
        return Ok(None);
    }
    serde_json::from_value(record.value.clone())
        .map(Some)
        .context("invalid site.standard.document record")
}

fn render_content_value(value: &Value) -> String {
    match value {
        Value::Array(items) => items.iter().map(render_content_value).collect::<String>(),
        Value::Object(fields) => render_content_object(fields),
        Value::String(text) => text_content_html(text),
        _ => String::new(),
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

fn render_content_object(fields: &serde_json::Map<String, Value>) -> String {
    let block_type = string_field(fields, &["$type", "type"]).to_ascii_lowercase();

    if let Some(block) = fields.get("block") {
        return render_content_value(block);
    }
    if block_type.contains("content") {
        return render_first_nested(fields, &["pages", "items", "blocks", "children", "content"]);
    }
    if block_type.contains("lineardocument") {
        return render_first_nested(fields, &["blocks", "children", "content"]);
    }
    if block_type.contains("unorderedlist")
        || block_type.contains("bulletlist")
        || block_type.contains("unordered-list")
    {
        return render_list(fields, false);
    }
    if block_type.contains("orderedlist") || block_type.contains("ordered-list") {
        return render_list(fields, true);
    }
    if block_type.contains("heading") || block_type.contains("header") {
        return render_heading(fields);
    }
    if block_type.contains("code") {
        return render_code(fields);
    }
    if block_type.contains("blockquote") || block_type.contains("quote") {
        let inner = if has_text(fields) {
            text_content_html(&text_field(fields))
        } else {
            render_first_nested(fields, &["content", "children", "blocks", "items"])
        };
        return (!inner.trim().is_empty())
            .then(|| format!("<blockquote>{inner}</blockquote>"))
            .unwrap_or_default();
    }
    if block_type.contains("horizontalrule") || block_type.contains("horizontal-rule") || block_type == "hr" {
        return "<hr>".to_string();
    }
    if block_type.contains("image") {
        return render_image(fields);
    }
    if has_text(fields) {
        return text_content_html(&text_field(fields));
    }

    render_first_nested(fields, &["content", "children", "pages", "blocks", "items"])
}

fn render_first_nested(fields: &serde_json::Map<String, Value>, names: &[&str]) -> String {
    names
        .iter()
        .find_map(|name| fields.get(*name))
        .map(render_content_value)
        .unwrap_or_default()
}

fn render_list(fields: &serde_json::Map<String, Value>, ordered: bool) -> String {
    let Some(items) = first_value(fields, &["children", "items", "content"]).and_then(Value::as_array) else {
        return String::new();
    };

    let tag = if ordered { "ol" } else { "ul" };
    let mut html = format!("<{tag}>");
    for item in items {
        let body = render_list_item(item);
        if !body.trim().is_empty() {
            html.push_str(&format!("<li>{body}</li>"));
        }
    }
    html.push_str(&format!("</{tag}>"));
    html
}

fn render_list_item(value: &Value) -> String {
    let Some(fields) = value.as_object() else {
        return text_content_html(&value.to_string());
    };

    let mut html = String::new();
    if let Some(content) = fields.get("content") {
        html.push_str(&render_content_value(content));
    } else if has_text(fields) {
        html.push_str(&escape_html(&text_field(fields)));
    }
    for key in ["children", "orderedListChildren", "unorderedListChildren"] {
        if let Some(value) = fields.get(key) {
            html.push_str(&render_content_value(value));
        }
    }
    html
}

fn render_heading(fields: &serde_json::Map<String, Value>) -> String {
    let text = text_field(fields);
    if text.is_empty() {
        return String::new();
    }
    let level = first_value(fields, &["level"])
        .and_then(Value::as_i64)
        .unwrap_or(2)
        .clamp(1, 6);
    format!("<h{level}>{}</h{level}>", escape_html(&text))
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

fn render_image(fields: &serde_json::Map<String, Value>) -> String {
    let attrs = fields.get("attrs").and_then(Value::as_object).unwrap_or(fields);
    let src = string_field(attrs, &["src", "url"]);
    if src.is_empty() {
        return String::new();
    }
    let alt = string_field(attrs, &["alt", "caption"]);
    format!(
        "<figure><img src=\"{}\" alt=\"{}\"></figure>",
        escape_html(&src),
        escape_html(&alt)
    )
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

fn has_text(fields: &serde_json::Map<String, Value>) -> bool {
    !text_field(fields).is_empty()
}

fn text_field(fields: &serde_json::Map<String, Value>) -> String {
    string_field(fields, &["plaintext", "text", "body", "title"])
}

fn string_field(fields: &serde_json::Map<String, Value>, names: &[&str]) -> String {
    first_value(fields, names)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn first_value<'a>(fields: &'a serde_json::Map<String, Value>, names: &[&str]) -> Option<&'a Value> {
    names.iter().find_map(|name| fields.get(*name))
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
            .map(decode_percentage)
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

fn decode_percentage(value: &str) -> Result<String> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                anyhow::bail!("incomplete percent escape");
            }
            let high = hex_value(bytes[index + 1])?;
            let low = hex_value(bytes[index + 2])?;
            output.push((high << 4) | low);
            index += 3;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(output).context("percent-decoded value is not UTF-8")
}

fn hex_value(byte: u8) -> Result<u8> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => anyhow::bail!("invalid percent escape"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
