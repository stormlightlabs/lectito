use crate::{Document, FetchConfig, LectitoError, Metadata, Result};
use std::future::Future;
use std::pin::Pin;
use url::Url;

pub enum ExtractorOutcome {
    Selector { selector: String },
    Html { content_html: String, metadata_patch: Metadata },
}

pub trait SiteExtractor: Send + Sync {
    fn name(&self) -> &'static str;
    fn matches(&self, url: &Url) -> bool;

    fn extract(&self, doc: &Document, url: &Url) -> Result<Option<ExtractorOutcome>>;

    #[cfg(feature = "fetch")]
    fn extract_async<'a>(
        &'a self, _doc: &'a Document, _url: &'a Url, _fetch_config: &'a FetchConfig,
    ) -> Pin<Box<dyn Future<Output = Result<Option<ExtractorOutcome>>> + Send + 'a>> {
        Box::pin(async { Ok(None) })
    }
}

pub struct ExtractorRegistry {
    extractors: Vec<Box<dyn SiteExtractor>>,
}

impl ExtractorRegistry {
    pub fn new() -> Self {
        Self {
            extractors: vec![
                Box::new(GitForgeExtractor),
                Box::new(RedditExtractor),
                Box::new(YouTubeExtractor),
                Box::new(HackerNewsExtractor),
                Box::new(SubstackExtractor),
            ],
        }
    }

    pub fn extract(&self, doc: &Document, url: &Url) -> Result<Option<ExtractorOutcome>> {
        for extractor in self.extractors.iter().filter(|extractor| extractor.matches(url)) {
            if let Some(outcome) = extractor.extract(doc, url)? {
                return Ok(Some(outcome));
            }
        }

        Ok(None)
    }

    #[cfg(feature = "fetch")]
    pub async fn extract_async(
        &self, doc: &Document, url: &Url, fetch_config: &FetchConfig,
    ) -> Result<Option<ExtractorOutcome>> {
        for extractor in self.extractors.iter().filter(|extractor| extractor.matches(url)) {
            if let Some(outcome) = extractor.extract(doc, url)? {
                return Ok(Some(outcome));
            }

            if let Some(outcome) = extractor.extract_async(doc, url, fetch_config).await? {
                return Ok(Some(outcome));
            }
        }

        Ok(None)
    }
}

impl Default for ExtractorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

struct GitForgeExtractor;
struct RedditExtractor;
struct HackerNewsExtractor;
struct SubstackExtractor;
struct YouTubeExtractor;

impl SiteExtractor for GitForgeExtractor {
    fn name(&self) -> &'static str {
        "git-forge"
    }

    fn matches(&self, url: &Url) -> bool {
        matches!(
            url.host_str().unwrap_or_default(),
            "github.com" | "gitlab.com" | "codeberg.org" | "tangled.org"
        )
    }

    fn extract(&self, doc: &Document, url: &Url) -> Result<Option<ExtractorOutcome>> {
        let path = url.path().to_ascii_lowercase();

        if path.contains("/issues/")
            || path.contains("/pull/")
            || path.contains("/merge_requests/")
            || path.contains("/merge-requests/")
        {
            return synthesize_thread(
                doc,
                url,
                &[
                    "[data-issue-title]",
                    ".issue-title",
                    ".js-issue-title",
                    "h1",
                ],
                &[
                    "[data-issue-body]",
                    ".issue-body",
                    ".comment-body",
                    ".description",
                    ".timeline-comment .comment-body",
                ],
                &["[data-comment]", ".timeline-comment", ".comment", ".discussion-item"],
                Some(host_site_name(url)),
            )
            .map(Some);
        }

        for selector in [
            "[data-readme-body]",
            "article.markdown-body",
            ".repository-content .markdown-body",
            ".markdown-body",
            ".blob-content",
            ".file-content",
            ".readme",
            "[data-file-body]",
        ] {
            if doc.select(selector).map(|els| !els.is_empty()).unwrap_or(false) {
                return Ok(Some(ExtractorOutcome::Selector {
                    selector: selector.to_string(),
                }));
            }
        }

        Ok(None)
    }
}

impl SiteExtractor for RedditExtractor {
    fn name(&self) -> &'static str {
        "reddit"
    }

    fn matches(&self, url: &Url) -> bool {
        url.host_str().unwrap_or_default().contains("reddit.com")
    }

    fn extract(&self, doc: &Document, _url: &Url) -> Result<Option<ExtractorOutcome>> {
        let title = first_text(doc, &["[data-post-title]", "h1", ".title a.title", ".post-title"])
            .or_else(|| doc.title())
            .unwrap_or_else(|| "Reddit discussion".to_string());
        let author = first_text(doc, &["[data-post-author]", ".author", "a.author"]);
        let date = first_attr(doc, &["time"], "datetime").or_else(|| first_text(doc, &["time"]));
        let post_body_html = first_inner_html(doc, &["[data-post-body]", ".usertext-body", ".md", ".post-body"]);
        let comments_html = build_comment_thread_html(
            doc,
            &["[data-comment]", ".comment"],
            &["[data-author]", ".author", "a.author"],
            &["time", ".live-timestamp"],
            &["[data-comment-body]", ".comment-body", ".usertext-body", ".md"],
        );

        let mut html = String::new();
        html.push_str("<article class=\"site-extractor reddit-thread\">");
        html.push_str(&format!("<h1>{}</h1>", escape_html(&title)));
        if let Some(author) = &author {
            html.push_str(&format!("<p><strong>{}</strong></p>", escape_html(author)));
        }
        if let Some(date) = &date {
            html.push_str(&format!("<time datetime=\"{}\">{}</time>", escape_html(date), escape_html(date)));
        }
        if let Some(post_body_html) = post_body_html {
            html.push_str("<section><h2>Post</h2>");
            html.push_str(&post_body_html);
            html.push_str("</section>");
        }
        if !comments_html.is_empty() {
            html.push_str("<section><h2>Comments</h2>");
            html.push_str(&comments_html);
            html.push_str("</section>");
        }
        html.push_str("</article>");

        Ok(Some(ExtractorOutcome::Html {
            content_html: html,
            metadata_patch: Metadata {
                title: Some(title),
                author,
                date,
                site_name: Some("Reddit".to_string()),
                ..Default::default()
            },
        }))
    }
}

impl SiteExtractor for HackerNewsExtractor {
    fn name(&self) -> &'static str {
        "hacker-news"
    }

    fn matches(&self, url: &Url) -> bool {
        url.host_str().unwrap_or_default() == "news.ycombinator.com"
            && url.path().eq_ignore_ascii_case("/item")
    }

    fn extract(&self, doc: &Document, _url: &Url) -> Result<Option<ExtractorOutcome>> {
        let title = first_text(doc, &["[data-story-title]", ".titleline a", "a.storylink", "title"])
            .or_else(|| doc.title())
            .unwrap_or_else(|| "Hacker News thread".to_string());
        let comments_html = build_comment_thread_html(
            doc,
            &["[data-comment]", ".comment"],
            &["[data-author]", ".hnuser", ".author"],
            &["time", "span.age"],
            &["[data-comment-body]", ".commtext", ".comment-body"],
        );

        let mut html = String::new();
        html.push_str("<article class=\"site-extractor hacker-news-thread\">");
        html.push_str(&format!("<h1>{}</h1>", escape_html(&title)));
        if !comments_html.is_empty() {
            html.push_str("<section><h2>Comments</h2>");
            html.push_str(&comments_html);
            html.push_str("</section>");
        }
        html.push_str("</article>");

        Ok(Some(ExtractorOutcome::Html {
            content_html: html,
            metadata_patch: Metadata {
                title: Some(title),
                site_name: Some("Hacker News".to_string()),
                ..Default::default()
            },
        }))
    }
}

impl SiteExtractor for SubstackExtractor {
    fn name(&self) -> &'static str {
        "substack"
    }

    fn matches(&self, url: &Url) -> bool {
        let host = url.host_str().unwrap_or_default();
        host.contains("substack.com") || host.ends_with(".substack.com")
    }

    fn extract(&self, doc: &Document, url: &Url) -> Result<Option<ExtractorOutcome>> {
        let path = url.path().to_ascii_lowercase();
        let is_note = path.contains("/note/") || path.contains("/notes/");

        if !is_note {
            for selector in [
                "[data-post-body]",
                "article.post",
                ".available-content",
                "article",
            ] {
                if doc.select(selector).map(|els| !els.is_empty()).unwrap_or(false) {
                    return Ok(Some(ExtractorOutcome::Selector {
                        selector: selector.to_string(),
                    }));
                }
            }
        }

        let title = first_text(doc, &["[data-note-title]", "h1"])
            .or_else(|| doc.title())
            .unwrap_or_else(|| "Substack note".to_string());
        let comments_html = build_comment_thread_html(
            doc,
            &["[data-note]", ".note-thread article", ".note-item"],
            &["[data-author]", ".author", ".user-name"],
            &["time"],
            &["[data-note-body]", ".note-body", ".body", ".available-content"],
        );

        let mut html = String::new();
        html.push_str("<article class=\"site-extractor substack-note\">");
        html.push_str(&format!("<h1>{}</h1>", escape_html(&title)));
        if !comments_html.is_empty() {
            html.push_str("<section><h2>Thread</h2>");
            html.push_str(&comments_html);
            html.push_str("</section>");
        }
        html.push_str("</article>");

        Ok(Some(ExtractorOutcome::Html {
            content_html: html,
            metadata_patch: Metadata {
                title: Some(title),
                site_name: Some("Substack".to_string()),
                ..Default::default()
            },
        }))
    }
}

impl SiteExtractor for YouTubeExtractor {
    fn name(&self) -> &'static str {
        "youtube"
    }

    fn matches(&self, url: &Url) -> bool {
        matches!(url.host_str().unwrap_or_default(), "youtube.com" | "www.youtube.com" | "youtu.be")
    }

    fn extract(&self, _doc: &Document, _url: &Url) -> Result<Option<ExtractorOutcome>> {
        Ok(None)
    }

    #[cfg(feature = "fetch")]
    fn extract_async<'a>(
        &'a self, doc: &'a Document, url: &'a Url, fetch_config: &'a FetchConfig,
    ) -> Pin<Box<dyn Future<Output = Result<Option<ExtractorOutcome>>> + Send + 'a>> {
        let title = first_text(doc, &["h1", "title"])
            .or_else(|| doc.title())
            .unwrap_or_else(|| "YouTube transcript".to_string());
        let page_html = doc.as_string();
        let embedded_transcript = first_inner_html(
            doc,
            &[
                "script[data-innertube-transcript]",
                "script#lectito-youtube-transcript",
            ],
        );
        let url = url.clone();
        let fetch_config = fetch_config.clone();

        Box::pin(async move {
            let transcript_json = if let Some(script_json) = embedded_transcript {
                Some(script_json)
            } else if let Some(base_url) = extract_youtube_caption_url(&page_html) {
                Some(crate::fetch::fetch_url(&base_url, &fetch_config).await?)
            } else {
                None
            };

            let Some(transcript_json) = transcript_json else {
                return Ok(None);
            };

            let transcript_html = render_youtube_transcript(&url, &title, &transcript_json)?;
            Ok(Some(ExtractorOutcome::Html {
                content_html: transcript_html,
                metadata_patch: Metadata {
                    title: Some(title),
                    site_name: Some("YouTube".to_string()),
                    ..Default::default()
                },
            }))
        })
    }
}

fn synthesize_thread(
    doc: &Document, url: &Url, title_selectors: &[&str], body_selectors: &[&str], comment_selectors: &[&str],
    site_name: Option<String>,
) -> Result<ExtractorOutcome> {
    let title = first_text(doc, title_selectors)
        .or_else(|| doc.title())
        .unwrap_or_else(|| "Discussion".to_string());
    let description_html = first_inner_html(doc, body_selectors).unwrap_or_default();
    let lead_author = first_text(doc, &["[data-author]", ".author", ".comment-author", ".user"]);
    let lead_date = first_attr(doc, &["time"], "datetime").or_else(|| first_text(doc, &["time"]));
    let mut comments_html = build_comment_thread_html(
        doc,
        comment_selectors,
        &["[data-author]", ".author", ".comment-author", ".user", "a.author"],
        &["time"],
        &["[data-comment-body]", ".comment-body", ".md", ".note-body", ".usertext-body"],
    );

    if !description_html.is_empty() {
        let description_text = strip_tags(&description_html);
        if let Some(first_comment_body) = first_text(doc, &["[data-comment-body]", ".comment-body", ".md", ".note-body"])
            && normalize_ws(&first_comment_body) == normalize_ws(&description_text)
        {
            comments_html = remove_first_article(&comments_html);
        }
    }

    let mut html = String::new();
    html.push_str("<article class=\"site-extractor thread\">");
    html.push_str(&format!("<h1>{}</h1>", escape_html(&title)));
    html.push_str(&format!("<p><a href=\"{}\">{}</a></p>", escape_html(url.as_str()), escape_html(url.as_str())));
    if !description_html.is_empty() {
        html.push_str("<section><h2>Description</h2>");
        html.push_str(&description_html);
        html.push_str("</section>");
    }
    if !comments_html.is_empty() {
        html.push_str("<section><h2>Comments</h2>");
        html.push_str(&comments_html);
        html.push_str("</section>");
    }
    html.push_str("</article>");

    Ok(ExtractorOutcome::Html {
        content_html: html,
        metadata_patch: Metadata {
            title: Some(title),
            author: lead_author,
            date: lead_date,
            site_name,
            ..Default::default()
        },
    })
}

fn build_comment_thread_html(
    doc: &Document, comment_selectors: &[&str], author_selectors: &[&str], time_selectors: &[&str], body_selectors: &[&str],
) -> String {
    let Some(selector) = comment_selectors
        .iter()
        .find(|selector| doc.select(selector).map(|els| !els.is_empty()).unwrap_or(false))
    else {
        return String::new();
    };

    let Ok(comments) = doc.select(selector) else {
        return String::new();
    };

    let mut rendered = Vec::new();
    for comment in comments {
        let author = first_text_in_element(&comment, author_selectors);
        let time = first_attr_in_element(&comment, time_selectors, "datetime")
            .or_else(|| first_text_in_element(&comment, time_selectors));
        let body_html = first_inner_html_in_element(&comment, body_selectors)
            .or_else(|| (!comment.text().trim().is_empty()).then(|| format!("<p>{}</p>", escape_html(&comment.text()))));

        let Some(body_html) = body_html else {
            continue;
        };

        let mut item = String::new();
        item.push_str("<article class=\"comment\">");
        if let Some(author) = &author {
            item.push_str(&format!("<h3>{}</h3>", escape_html(author)));
        }
        if let Some(time) = &time {
            item.push_str(&format!("<time datetime=\"{}\">{}</time>", escape_html(time), escape_html(time)));
        }
        item.push_str("<div class=\"comment-body\">");
        item.push_str(&body_html);
        item.push_str("</div></article>");

        let depth = comment
            .attr("data-depth")
            .or_else(|| comment.attr("data-level"))
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        rendered.push(wrap_blockquotes(item, depth.min(4)));
    }

    rendered.join("\n")
}

fn wrap_blockquotes(mut html: String, depth: usize) -> String {
    for _ in 0..depth {
        html = format!("<blockquote>{html}</blockquote>");
    }
    html
}

fn remove_first_article(html: &str) -> String {
    if let Some(start) = html.find("<article")
        && let Some(end) = html[start..].find("</article>")
    {
        let mut trimmed = html.to_string();
        trimmed.replace_range(start..start + end + "</article>".len(), "");
        return trimmed.trim().to_string();
    }

    html.to_string()
}

fn first_text(doc: &Document, selectors: &[&str]) -> Option<String> {
    selectors.iter().find_map(|selector| {
        doc.select(selector)
            .ok()
            .and_then(|elements| elements.into_iter().next())
            .map(|element| element.text().trim().to_string())
            .filter(|text| !text.is_empty())
    })
}

fn first_attr(doc: &Document, selectors: &[&str], attr: &str) -> Option<String> {
    selectors.iter().find_map(|selector| {
        doc.select(selector)
            .ok()
            .and_then(|elements| elements.into_iter().next())
            .and_then(|element| element.attr(attr).map(ToString::to_string))
            .filter(|value| !value.is_empty())
    })
}

fn first_inner_html(doc: &Document, selectors: &[&str]) -> Option<String> {
    selectors.iter().find_map(|selector| {
        doc.select(selector)
            .ok()
            .and_then(|elements| elements.into_iter().next())
            .map(|element| element.inner_html())
            .filter(|html| !html.trim().is_empty())
    })
}

fn first_text_in_element(element: &crate::parse::Element<'_>, selectors: &[&str]) -> Option<String> {
    selectors.iter().find_map(|selector| {
        element
            .select(selector)
            .ok()
            .and_then(|elements| elements.into_iter().next())
            .map(|child| child.text().trim().to_string())
            .filter(|text| !text.is_empty())
    })
}

fn first_attr_in_element(element: &crate::parse::Element<'_>, selectors: &[&str], attr: &str) -> Option<String> {
    selectors.iter().find_map(|selector| {
        element
            .select(selector)
            .ok()
            .and_then(|elements| elements.into_iter().next())
            .and_then(|child| child.attr(attr).map(ToString::to_string))
            .filter(|value| !value.is_empty())
    })
}

fn first_inner_html_in_element(element: &crate::parse::Element<'_>, selectors: &[&str]) -> Option<String> {
    selectors.iter().find_map(|selector| {
        element
            .select(selector)
            .ok()
            .and_then(|elements| elements.into_iter().next())
            .map(|child| child.inner_html())
            .filter(|html| !html.trim().is_empty())
    })
}

fn host_site_name(url: &Url) -> String {
    match url.host_str().unwrap_or_default() {
        "github.com" => "GitHub",
        "gitlab.com" => "GitLab",
        "codeberg.org" => "Codeberg",
        "tangled.org" => "Tangled",
        other => other,
    }
    .to_string()
}

fn extract_youtube_caption_url(html: &str) -> Option<String> {
    static BASE_URL_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = BASE_URL_RE.get_or_init(|| regex::Regex::new(r#""baseUrl":"([^"]+)""#).unwrap());
    let value = re.captures(html)?.get(1)?.as_str();
    Some(
        value
            .replace("\\u0026", "&")
            .replace("\\/", "/")
            .replace("&amp;", "&"),
    )
}

fn render_youtube_transcript(url: &Url, title: &str, transcript_json: &str) -> Result<String> {
    let value: serde_json::Value =
        serde_json::from_str(transcript_json).map_err(|e| LectitoError::HtmlParseError(e.to_string()))?;
    let events = value
        .get("events")
        .and_then(|events| events.as_array())
        .ok_or_else(|| LectitoError::NoContent)?;

    let mut paragraphs = Vec::new();
    let mut current = Vec::new();
    let mut current_start = None;

    for event in events {
        let start_ms = event.get("tStartMs").and_then(|value| value.as_u64()).unwrap_or(0);
        let Some(segs) = event.get("segs").and_then(|value| value.as_array()) else {
            continue;
        };

        let mut line = String::new();
        for seg in segs {
            if let Some(text) = seg.get("utf8").and_then(|value| value.as_str()) {
                line.push_str(text);
            }
        }

        let line = normalize_ws(&line);
        if line.is_empty() {
            continue;
        }

        if current_start.is_none() {
            current_start = Some(start_ms);
        }
        current.push(line);

        if current.len() >= 3 {
            paragraphs.push((current_start.unwrap_or(0), current.join(" ")));
            current.clear();
            current_start = None;
        }
    }

    if !current.is_empty() {
        paragraphs.push((current_start.unwrap_or(0), current.join(" ")));
    }

    if paragraphs.is_empty() {
        return Err(LectitoError::NoContent);
    }

    let mut html = String::new();
    html.push_str("<article class=\"site-extractor youtube-transcript\">");
    html.push_str(&format!("<h1>{}</h1>", escape_html(title)));
    html.push_str("<section><h2>Transcript</h2>");
    for (start_ms, text) in paragraphs {
        let seconds = start_ms / 1000;
        html.push_str(&format!(
            "<p><a href=\"{}&t={}s\">[{:#02}:{:#02}]</a> {}</p>",
            escape_html(url.as_str()),
            seconds,
            seconds / 60,
            seconds % 60,
            escape_html(&text)
        ));
    }
    html.push_str("</section></article>");

    Ok(html)
}

fn normalize_ws(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_tags(value: &str) -> String {
    static TAG_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    TAG_RE
        .get_or_init(|| regex::Regex::new(r"<[^>]+>").unwrap())
        .replace_all(value, " ")
        .to_string()
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_prefers_git_forge_issue_extractor() {
        let registry = ExtractorRegistry::new();
        let url = Url::parse("https://github.com/example/repo/issues/42").unwrap();
        let doc = Document::parse(
            r#"<html><body><h1 class="issue-title">Broken parser</h1><div class="issue-body"><p>Lead</p></div></body></html>"#,
        )
        .unwrap();

        let outcome = registry.extract(&doc, &url).unwrap();
        assert!(matches!(outcome, Some(ExtractorOutcome::Html { .. })));
    }

    #[test]
    fn test_registry_returns_selector_for_readme() {
        let registry = ExtractorRegistry::new();
        let url = Url::parse("https://github.com/example/repo").unwrap();
        let doc = Document::parse(
            r#"<html><body><article class="markdown-body"><h1>README</h1><p>Hello</p></article></body></html>"#,
        )
        .unwrap();

        let outcome = registry.extract(&doc, &url).unwrap();
        assert!(matches!(outcome, Some(ExtractorOutcome::Selector { .. })));
    }

    #[test]
    fn test_render_youtube_transcript_groups_segments() {
        let html = render_youtube_transcript(
            &Url::parse("https://www.youtube.com/watch?v=demo").unwrap(),
            "Demo",
            r#"{"events":[{"tStartMs":0,"segs":[{"utf8":"Hello "} ]},{"tStartMs":1000,"segs":[{"utf8":"world"}]},{"tStartMs":2000,"segs":[{"utf8":" again"}]}]}"#,
        )
        .unwrap();

        assert!(html.contains("Transcript"));
        assert!(html.contains("Hello world again"));
    }
}
