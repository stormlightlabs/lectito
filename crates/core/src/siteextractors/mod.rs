#[cfg(feature = "fetch")]
use crate::LectitoError;
use crate::{Document, FetchConfig, Metadata, Result, utils};
#[cfg(feature = "fetch")]
use std::future::Future;
#[cfg(feature = "fetch")]
use std::pin::Pin;
use url::Url;

pub struct HtmlExtractorOutcome {
    pub content_html: String,
    pub metadata_patch: Metadata,
}

pub enum ExtractorOutcome {
    Selector { selector: String },
    Html(Box<HtmlExtractorOutcome>),
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
                Box::new(DocsRsExtractor),
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

    #[cfg(not(feature = "fetch"))]
    pub async fn extract_async(
        &self, doc: &Document, url: &Url, _fetch_config: &FetchConfig,
    ) -> Result<Option<ExtractorOutcome>> {
        self.extract(doc, url)
    }
}

impl Default for ExtractorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

struct GitForgeExtractor;
struct DocsRsExtractor;
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
            let (title_selectors, body_selectors, comment_selectors) = match url.host_str().unwrap_or_default() {
                "gitlab.com" => (
                    vec![
                        "meta[property='og:title']",
                        ".merge-request-sticky-title",
                        ".js-mr-header + a",
                        "h1",
                    ],
                    vec![
                        ".detail-page-description .description .md",
                        ".detail-page-description .description",
                        ".description .md",
                    ],
                    vec![".timeline-entry.note", ".note"],
                ),
                "codeberg.org" => (
                    vec!["meta[property='og:title']", ".issue-title > a", ".issue-title", "h1"],
                    vec![
                        ".timeline-item.comment.first .comment-body",
                        ".comment-list .timeline-item.comment.first .comment-body",
                    ],
                    vec![".timeline-item.comment:not(.first)", ".timeline-item.comment"],
                ),
                _ => (
                    vec![
                        "meta[property='og:title']",
                        "[data-issue-title]",
                        ".issue-title a",
                        ".issue-title",
                        ".js-issue-title",
                        "h1",
                    ],
                    vec![
                        "[data-issue-body]",
                        ".issue-body",
                        ".discussion-item-body .comment-body",
                        ".comment-body",
                        ".description",
                    ],
                    vec!["[data-comment]", ".timeline-comment", ".discussion-item"],
                ),
            };

            return synthesize_thread(
                doc,
                url,
                &title_selectors,
                &body_selectors,
                &comment_selectors,
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
                return Ok(Some(ExtractorOutcome::Selector { selector: selector.to_string() }));
            }
        }

        Ok(None)
    }
}

impl SiteExtractor for DocsRsExtractor {
    fn name(&self) -> &'static str {
        "docs-rs"
    }

    fn matches(&self, url: &Url) -> bool {
        matches!(url.host_str().unwrap_or_default(), "docs.rs" | "www.docs.rs")
    }

    fn extract(&self, doc: &Document, _url: &Url) -> Result<Option<ExtractorOutcome>> {
        for selector in [
            "#main-content",
            "main > .width-limiter > #main-content",
            "main > .width-limiter > section.content",
            "main section.content",
        ] {
            if doc.select(selector).map(|els| !els.is_empty()).unwrap_or(false) {
                return Ok(Some(ExtractorOutcome::Selector { selector: selector.to_string() }));
            }
        }

        Ok(None)
    }

    #[cfg(feature = "fetch")]
    fn extract_async<'a>(
        &'a self, _doc: &'a Document, _url: &'a Url, _fetch_config: &'a FetchConfig,
    ) -> Pin<Box<dyn Future<Output = Result<Option<ExtractorOutcome>>> + Send + 'a>> {
        Box::pin(async { Ok(None) })
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
        if let Some(post) = first_element(doc, &["#siteTable > .thing.link", "#siteTable .thing.link"]) {
            let title = first_attr(doc, &["meta[property='og:title']"], "content")
                .or_else(|| {
                    first_text_in_element(
                        &post,
                        &[
                            ".top-matter .title a.title",
                            ".top-matter .title",
                            "[data-post-title]",
                            "h1",
                        ],
                    )
                })
                .or_else(|| doc.title())
                .unwrap_or_else(|| "Reddit discussion".to_string());
            let author = first_text_in_element(
                &post,
                &[".tagline .author", "[data-post-author]", ".author", "a.author"],
            );
            let date = first_attr_in_element(&post, &["time", ".live-timestamp"], "datetime")
                .or_else(|| first_text_in_element(&post, &["time", ".live-timestamp"]));
            let post_body_html = first_inner_html_in_element(
                &post,
                &["[data-post-body]", ".expando .usertext-body .md", ".usertext-body .md"],
            );
            let comments_html = build_comment_thread_html(
                doc,
                &[".commentarea .thing.comment", "[data-comment]"],
                &[".tagline .author", "[data-author]", ".author", "a.author"],
                &["time", ".live-timestamp"],
                &[".usertext-body .md", "[data-comment-body]", ".comment-body"],
            );

            return Ok(Some(render_reddit_thread(
                title,
                author,
                date,
                post_body_html,
                comments_html,
            )));
        }

        let title = first_attr(doc, &["meta[property='og:title']"], "content")
            .or_else(|| first_text(doc, &["[data-post-title]", "h1", ".title a.title", ".post-title"]))
            .or_else(|| doc.title())
            .unwrap_or_else(|| "Reddit discussion".to_string());
        let author = first_text(doc, &["[data-post-author]", ".author", "a.author"]);
        let date = first_attr(doc, &["time"], "datetime").or_else(|| first_text(doc, &["time"]));
        let post_body_html = first_inner_html(
            doc,
            &["[data-post-body]", ".usertext-body .md", ".usertext-body", ".post-body"],
        );
        let comments_html = build_comment_thread_html(
            doc,
            &["[data-comment]", ".thing.comment"],
            &["[data-author]", ".author", "a.author"],
            &["time", ".live-timestamp"],
            &[
                "[data-comment-body]",
                ".comment-body",
                ".usertext-body .md",
                ".usertext-body",
            ],
        );

        Ok(Some(render_reddit_thread(
            title,
            author,
            date,
            post_body_html,
            comments_html,
        )))
    }
}

fn render_reddit_thread(
    title: String, author: Option<String>, date: Option<String>, post_body_html: Option<String>, comments_html: String,
) -> ExtractorOutcome {
    let post_body_html = post_body_html
        .map(|html| sanitize_extracted_html(&html))
        .filter(|html| !utils::normalize_whitespace(&strip_tags(html)).is_empty());
    let comments_html = sanitize_extracted_html(&comments_html);

    let mut html = String::new();
    html.push_str("<article class=\"site-extractor reddit-thread\">");
    html.push_str(&format!("<h1>{}</h1>", escape_html(&title)));
    if let Some(author) = &author {
        html.push_str(&format!("<p><strong>{}</strong></p>", escape_html(author)));
    }
    if let Some(date) = &date {
        html.push_str(&format!(
            "<time datetime=\"{}\">{}</time>",
            escape_html(date),
            escape_html(date)
        ));
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

    ExtractorOutcome::Html(Box::new(HtmlExtractorOutcome {
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

impl SiteExtractor for HackerNewsExtractor {
    fn name(&self) -> &'static str {
        "hacker-news"
    }

    fn matches(&self, url: &Url) -> bool {
        url.host_str().unwrap_or_default() == "news.ycombinator.com" && url.path().eq_ignore_ascii_case("/item")
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

        Ok(Some(ExtractorOutcome::Html(Box::new(HtmlExtractorOutcome {
            content_html: html,
            metadata_patch: Metadata {
                title: Some(title),
                site_name: Some("Hacker News".to_string()),
                ..Default::default()
            },
        }))))
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
                ".available-content .body.markup",
                ".available-content",
                "[data-post-body]",
                "article.post",
                "article",
            ] {
                if let Some(element) = first_element(doc, &[selector]) {
                    return Ok(Some(ExtractorOutcome::Html(Box::new(HtmlExtractorOutcome {
                        content_html: sanitize_extracted_html(&element.outer_html()),
                        metadata_patch: Metadata {
                            title: first_attr(
                                doc,
                                &["meta[property='og:title']", "meta[name='twitter:title']"],
                                "content",
                            )
                            .or_else(|| first_text(doc, &["h1"]))
                            .or_else(|| doc.title()),
                            author: first_text(doc, &[".byline-wrapper a", ".author-name", ".author"]),
                            date: first_attr(doc, &["time"], "datetime").or_else(|| first_text(doc, &["time"])),
                            site_name: Some("Substack".to_string()),
                            ..Default::default()
                        },
                    }))));
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

        Ok(Some(ExtractorOutcome::Html(Box::new(HtmlExtractorOutcome {
            content_html: html,
            metadata_patch: Metadata {
                title: Some(title),
                site_name: Some("Substack".to_string()),
                ..Default::default()
            },
        }))))
    }
}

impl SiteExtractor for YouTubeExtractor {
    fn name(&self) -> &'static str {
        "youtube"
    }

    fn matches(&self, url: &Url) -> bool {
        matches!(
            url.host_str().unwrap_or_default(),
            "youtube.com" | "www.youtube.com" | "youtu.be"
        )
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
            &["script[data-innertube-transcript]", "script#lectito-youtube-transcript"],
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
            Ok(Some(ExtractorOutcome::Html(Box::new(HtmlExtractorOutcome {
                content_html: transcript_html,
                metadata_patch: Metadata {
                    title: Some(title),
                    site_name: Some("YouTube".to_string()),
                    ..Default::default()
                },
            }))))
        })
    }
}

fn synthesize_thread(
    doc: &Document, url: &Url, title_selectors: &[&str], body_selectors: &[&str], comment_selectors: &[&str],
    site_name: Option<String>,
) -> Result<ExtractorOutcome> {
    let title = first_text(doc, title_selectors)
        .or_else(|| doc.title())
        .map(|title| clean_thread_title(&title))
        .unwrap_or_else(|| "Discussion".to_string());
    let description_html = first_inner_html(doc, body_selectors)
        .map(|html| sanitize_extracted_html(&html))
        .unwrap_or_default();
    let lead_author = first_text(doc, &["[data-author]", ".author", ".comment-author", ".user"]);
    let lead_date = first_attr(doc, &["time"], "datetime").or_else(|| first_text(doc, &["time"]));
    let mut comments_html = sanitize_extracted_html(&build_comment_thread_html(
        doc,
        comment_selectors,
        &["[data-author]", ".author", ".comment-author", ".user", "a.author"],
        &["time", ".live-timestamp", "relative-time"],
        &[
            "[data-comment-body]",
            ".comment-body .render-content",
            ".comment-body",
            ".note-body",
            ".usertext-body .md",
            ".md",
        ],
    ));

    if !description_html.is_empty() {
        let description_text = strip_tags(&description_html);
        if let Some(first_comment_body) =
            first_text(doc, &["[data-comment-body]", ".comment-body", ".md", ".note-body"])
            && utils::normalize_whitespace(&first_comment_body) == utils::normalize_whitespace(&description_text)
        {
            comments_html = remove_first_article(&comments_html);
        }
    }

    let mut html = String::new();
    html.push_str("<article class=\"site-extractor thread\">");
    html.push_str(&format!("<h1>{}</h1>", escape_html(&title)));
    html.push_str(&format!(
        "<p><a href=\"{}\">{}</a></p>",
        escape_html(url.as_str()),
        escape_html(url.as_str())
    ));
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

    Ok(ExtractorOutcome::Html(Box::new(HtmlExtractorOutcome {
        content_html: html,
        metadata_patch: Metadata {
            title: Some(title),
            author: lead_author,
            date: lead_date,
            site_name,
            ..Default::default()
        },
    })))
}

fn build_comment_thread_html(
    doc: &Document, comment_selectors: &[&str], author_selectors: &[&str], time_selectors: &[&str],
    body_selectors: &[&str],
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
        let body_html =
            first_inner_html_in_element(&comment, body_selectors).map(|html| sanitize_extracted_html(&html));

        let Some(body_html) = body_html else {
            continue;
        };
        if utils::normalize_whitespace(&strip_tags(&body_html)).is_empty() {
            continue;
        }

        let mut item = String::new();
        item.push_str("<article class=\"comment\">");
        if let Some(author) = &author {
            item.push_str(&format!("<h3>{}</h3>", escape_html(author)));
        }
        if let Some(time) = &time {
            item.push_str(&format!(
                "<time datetime=\"{}\">{}</time>",
                escape_html(time),
                escape_html(time)
            ));
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

fn first_element<'a>(doc: &'a Document, selectors: &[&str]) -> Option<crate::parse::Element<'a>> {
    selectors.iter().find_map(|selector| {
        doc.select(selector)
            .ok()
            .and_then(|elements| elements.into_iter().next())
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

#[cfg(feature = "fetch")]
fn extract_youtube_caption_url(html: &str) -> Option<String> {
    static BASE_URL_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = BASE_URL_RE.get_or_init(|| regex::Regex::new(r#""baseUrl":"([^"]+)""#).unwrap());
    let value = re.captures(html)?.get(1)?.as_str();
    Some(value.replace("\\u0026", "&").replace("\\/", "/").replace("&amp;", "&"))
}

#[cfg(feature = "fetch")]
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

        let line = utils::normalize_whitespace(&line);
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

fn clean_thread_title(value: &str) -> String {
    let value = utils::normalize_whitespace(value);
    let value = value.strip_suffix("New issue").map(str::trim).unwrap_or(&value);
    value.to_string()
}

fn sanitize_extracted_html(value: &str) -> String {
    static REMOVALS: std::sync::OnceLock<Vec<regex::Regex>> = std::sync::OnceLock::new();
    static REPLACEMENTS: std::sync::OnceLock<Vec<(regex::Regex, &'static str)>> = std::sync::OnceLock::new();
    let removals = REMOVALS.get_or_init(|| {
        vec![
            regex::Regex::new(r#"(?is)<textarea[^>]*>.*?</textarea>"#).unwrap(),
            regex::Regex::new(r#"(?is)<(?:form|button)[^>]*>.*?</(?:form|button)>"#).unwrap(),
            regex::Regex::new(r#"(?is)<ul[^>]*class="[^"]*flat-list buttons[^"]*"[^>]*>.*?</ul>"#).unwrap(),
            regex::Regex::new(r#"(?is)<(?:div|section|aside)[^>]*class="[^"]*(?:raw-content|edit-content-zone|tw-hidden|hidden js-task-list-field|reportform|commentsignupbar|subscription-widget-wrap|subscription-widget|image-link-expand)[^"]*"[^>]*>.*?</(?:div|section|aside)>"#).unwrap(),
            regex::Regex::new(r#"(?is)<a[^>]*class="[^"]*embed-comment[^"]*"[^>]*>.*?</a>"#).unwrap(),
            regex::Regex::new(r#"(?is)<svg[^>]*>.*?</svg>"#).unwrap(),
            regex::Regex::new(r#"(?is)<p>\s*</p>"#).unwrap(),
        ]
    });
    let replacements = REPLACEMENTS.get_or_init(|| {
        vec![
            (
                regex::Regex::new(
                    r#"(?is)<h2([^>]*)>([^<]+?)<div[^>]*class="[^"]*header-anchor-parent[^"]*"[^>]*>.*?</h2>"#,
                )
                .unwrap(),
                "<h2$1>$2</h2>",
            ),
            (
                regex::Regex::new(r#"(?is)<div[^>]*class="[^"]*subscribe-widget[^"]*"[^>]*>.*$"#).unwrap(),
                "",
            ),
        ]
    });

    let mut cleaned = value.to_string();
    for re in removals {
        cleaned = re.replace_all(&cleaned, "").to_string();
    }
    for (re, replacement) in replacements {
        cleaned = re.replace_all(&cleaned, *replacement).to_string();
    }

    cleaned.trim().to_string()
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
        assert!(matches!(outcome, Some(ExtractorOutcome::Html(_))));
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

    #[cfg(feature = "fetch")]
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

    #[test]
    fn test_docs_rs_extractor_prefers_main_content_selector() {
        let extractor = DocsRsExtractor;
        let url = Url::parse("https://docs.rs/clap/latest/clap/struct.Command.html").unwrap();
        let doc = Document::parse(
            r#"
            <html>
              <body class="rustdoc-page">
                <nav class="sidebar"><p>Sidebar chrome</p></nav>
                <main>
                  <div class="width-limiter">
                    <section id="main-content" class="content">
                      <h1>Struct Command</h1>
                      <p>Build a command-line interface.</p>
                    </section>
                  </div>
                </main>
              </body>
            </html>
            "#,
        )
        .unwrap();
        let outcome = extractor
            .extract(&doc, &url)
            .unwrap()
            .expect("docs.rs extractor should return a selector");

        match outcome {
            ExtractorOutcome::Selector { selector } => {
                assert_eq!(selector, "#main-content");
            }
            ExtractorOutcome::Html(_) => panic!("expected selector outcome"),
        }
    }
}
