use lectito::escape_html;
use serde_json::Value;

use super::{AtUri, first_value, string_field};

pub fn is_standard_site_post(block_type: &str) -> bool {
    block_type.contains("standardsitepost") || block_type.contains("standard-site-post")
}

pub fn is_bsky_post(block_type: &str) -> bool {
    block_type.contains("bskypost") || block_type.contains("blueskypost")
}

pub fn is_website(block_type: &str) -> bool {
    block_type.contains("website") || block_type.contains("bookmark")
}

pub fn is_iframe(block_type: &str) -> bool {
    block_type.contains("iframe") || block_type.contains("embed")
}

pub fn is_button(block_type: &str) -> bool {
    block_type.contains("button")
}

pub fn render_standard_site_post(fields: &serde_json::Map<String, Value>) -> String {
    let uri = string_field(fields, &["uri", "atURI"]);
    if uri.is_empty() {
        return String::new();
    }
    format!(
        "<blockquote><p>Embedded Standard.site post: <a href=\"{}\">{}</a></p></blockquote>",
        escape_html(&uri),
        escape_html(&uri)
    )
}

pub fn render_bsky_post(fields: &serde_json::Map<String, Value>) -> String {
    let ref_fields = fields.get("postRef").and_then(Value::as_object).unwrap_or(fields);
    let uri = string_field(ref_fields, &["uri"]);
    let url = bsky_post_url(&uri, &string_field(fields, &["clientHost"])).unwrap_or(uri);
    if url.is_empty() {
        return String::new();
    }
    format!(
        "<blockquote><p>Embedded Bluesky post: <a href=\"{}\">{}</a></p></blockquote>",
        escape_html(&url),
        escape_html(&url)
    )
}

pub fn render_website<F>(fields: &serde_json::Map<String, Value>, blob_url: F) -> String
where
    F: Fn(&Value) -> Option<String>,
{
    let url = string_field(fields, &["src", "url", "href"]);
    if url.is_empty() {
        return String::new();
    }

    let title = string_field(fields, &["title", "text"]);
    let title = if title.is_empty() { &url } else { &title };
    let description = string_field(fields, &["description"]);
    let image = first_value(fields, &["previewImage", "image", "blob"]).and_then(blob_url);
    let mut html = format!("<p><a href=\"{}\">{}</a></p>", escape_html(&url), escape_html(title));
    if !description.is_empty() {
        html.push_str(&format!("<p>{}</p>", escape_html(&description)));
    }
    if let Some(image) = image {
        html.push_str(&format!(
            "<figure><img src=\"{}\" alt=\"\"></figure>",
            escape_html(&image)
        ));
    }
    html
}

pub fn render_iframe(fields: &serde_json::Map<String, Value>) -> String {
    let url = string_field(fields, &["url", "src"]);
    if url.is_empty() {
        return String::new();
    }
    format!(
        "<iframe src=\"{}\" loading=\"lazy\" referrerpolicy=\"no-referrer-when-downgrade\"></iframe>",
        escape_html(&url)
    )
}

pub fn render_button(fields: &serde_json::Map<String, Value>) -> String {
    let url = string_field(fields, &["url", "href"]);
    if url.is_empty() {
        return String::new();
    }
    let text = string_field(fields, &["text", "title"]);
    let text = if text.is_empty() { &url } else { &text };
    format!("<p><a href=\"{}\">{}</a></p>", escape_html(&url), escape_html(text))
}

fn bsky_post_url(at_uri: &str, client_host: &str) -> Option<String> {
    let parsed = AtUri::parse(at_uri).ok()?;
    if parsed.collection != "app.bsky.feed.post" {
        return None;
    }
    let host = if client_host.trim().is_empty() { "bsky.app" } else { client_host.trim() };
    Some(format!(
        "https://{}/profile/{}/post/{}",
        host.trim_start_matches("https://").trim_start_matches("http://"),
        parsed.authority,
        parsed.rkey
    ))
}
