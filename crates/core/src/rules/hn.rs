use kuchiki::NodeRef;
use kuchiki::traits::TendrilSink;
use url::Url;

use super::SiteExtractor;
use super::{dom, serialize};
use crate::config::ReadabilityOptions;
use crate::error::Result;
use crate::extract::ExtractAttempt;
use crate::metadata::{Metadata, decode_html_entities};
use crate::shared::escape_html;

struct HnComment {
    author: String,
    date: Option<String>,
    url: String,
    content_html: String,
    replies: Vec<HnComment>,
}

impl HnComment {
    fn to_html(&self) -> String {
        let mut html = String::from("<blockquote>");
        html.push_str("<p><small>");
        html.push_str(&escape_html(&self.author));
        if let Some(date) = &self.date {
            html.push_str(" · ");
            html.push_str(&escape_html(date));
        }
        html.push_str(" · ");
        html.push_str(&format!(r#"<a href="{}">link</a>"#, escape_html(&self.url)));
        html.push_str("</small></p>");
        html.push_str(&self.content_html);
        for reply in &self.replies {
            html.push_str(&reply.to_html());
        }
        html.push_str("</blockquote>");
        html
    }
}

pub struct HackerNewsExtractor;

impl SiteExtractor for HackerNewsExtractor {
    fn name(&self) -> &'static str {
        "hacker-news"
    }

    fn matches(&self, url: &Url) -> bool {
        url.host_str()
            .is_some_and(|host| host.trim_start_matches("www.") == "news.ycombinator.com")
    }

    fn extract(
        &self, document: &NodeRef, url: &Url, _: &ReadabilityOptions, metadata: &Metadata,
    ) -> Result<Option<ExtractAttempt>> {
        Self::try_thread(document, url, metadata)
    }
}

impl HackerNewsExtractor {
    fn try_thread(document: &NodeRef, url: &Url, metadata: &Metadata) -> Result<Option<ExtractAttempt>> {
        let Some(main_post) = dom::select_nodes(document, ".fatitem").into_iter().next() else {
            return Self::try_listing(document, url, metadata);
        };

        let mut metadata = metadata.clone();
        let title = Self::hn_post_title(&main_post)
            .or_else(|| metadata.title.clone().map(|title| title.replace(" | Hacker News", "")))
            .unwrap_or_else(|| "Hacker News".to_string());
        let author = Self::hn_post_author(&main_post);
        let published = Self::hn_post_date(&main_post);

        metadata.title = Some(title.clone());
        metadata.byline = author.clone();
        metadata.published_time = published.clone();
        metadata.site_name = Some("Hacker News".to_string());
        metadata.excerpt = Some(match author.as_deref().filter(|author| !author.is_empty()) {
            Some(author) => format!("{title} - by {author} on Hacker News"),
            None => format!("{title} - on Hacker News"),
        });

        let mut content = String::from(r#"<div id="readability-page-1" class="page">"#);
        content.push_str(r#"<article data-source="hackernews">"#);
        content.push_str(&format!("<h1>{}</h1>", escape_html(&title)));

        let story_url = Self::hn_story_url(&main_post);
        if let Some(story_url) = &story_url {
            content.push_str(&format!(
                r#"<p><a href="{}">{}</a></p>"#,
                escape_html(story_url),
                escape_html(story_url)
            ));
        }

        let meta = Self::hn_meta_line(&main_post, author.as_deref(), published.as_deref());
        if !meta.is_empty() {
            content.push_str(&format!("<p><small>{}</small></p>", escape_html(&meta)));
        }

        if let Some(toptext) = dom::select_nodes(&main_post, ".toptext").into_iter().next() {
            let html = serialize::serialize_children(&toptext)?;
            if !dom::inner_text(&toptext).is_empty() {
                content.push_str(r#"<div class="post-text">"#);
                content.push_str(&html);
                content.push_str("</div>");
            }
        }

        let comments = Self::hn_comments(document)?;
        if !comments.is_empty() {
            content.push_str("<h2>Comments</h2>");
            for comment in comments {
                content.push_str(&comment.to_html());
            }
        }

        content.push_str("</article></div>");
        let fragment = kuchiki::parse_html().one(format!("<html><body>{content}</body></html>"));
        let text_content = serialize::text_content(&dom::select_nodes(&fragment, "body"));
        let text_len = text_content.encode_utf16().count();

        Ok(Some(ExtractAttempt { metadata, content, text_content, text_len }))
    }

    fn try_listing(document: &NodeRef, url: &Url, metadata: &Metadata) -> Result<Option<ExtractAttempt>> {
        let stories = dom::select_nodes(document, "tr.athing");
        if stories.len() < 2 {
            return Ok(None);
        }

        let mut metadata = metadata.clone();
        let title = metadata
            .title
            .clone()
            .map(|title| title.replace(" | Hacker News", ""))
            .filter(|title| !title.trim().is_empty())
            .unwrap_or_else(|| "Hacker News".to_string());
        metadata.title = Some(title.clone());
        metadata.site_name = Some("Hacker News".to_string());
        metadata.excerpt = Some(format!("{title} stories on Hacker News"));

        let mut content = String::from(r#"<div id="readability-page-1" class="page">"#);
        content.push_str(r#"<article data-source="hackernews-listing">"#);
        content.push_str(&format!("<h1>{}</h1><ol>", escape_html(&title)));

        for row in stories {
            let Some(title_link) = dom::select_nodes(&row, ".titleline a[href]").into_iter().next() else {
                continue;
            };
            let story_title = dom::inner_text(&title_link);
            if story_title.is_empty() {
                continue;
            }
            let story_url = dom::attr(&title_link, "href")
                .and_then(|href| Self::absolute_or_original(url, &href))
                .unwrap_or_default();
            let site = dom::select_nodes(&row, ".sitestr")
                .into_iter()
                .next()
                .map(|node| dom::inner_text(&node))
                .filter(|site| !site.is_empty());
            let subtext = Self::next_element_sibling(&row, "tr");
            let score = subtext
                .as_ref()
                .and_then(|node| dom::select_nodes(node, ".score").into_iter().next())
                .map(|node| dom::inner_text(&node))
                .filter(|score| !score.is_empty());
            let author = subtext
                .as_ref()
                .and_then(|node| dom::select_nodes(node, ".hnuser").into_iter().next())
                .map(|node| dom::inner_text(&node))
                .filter(|author| !author.is_empty());
            let comment_url = dom::attr(&row, "id")
                .and_then(|id| Self::absolute_or_original(url, &format!("item?id={id}")))
                .unwrap_or_default();
            let comments = subtext
                .as_ref()
                .map(Self::hn_listing_comment_text)
                .filter(|comments| !comments.is_empty());

            content.push_str("<li>");
            content.push_str(&format!(
                r#"<a href="{}">{}</a>"#,
                escape_html(&story_url),
                escape_html(&story_title)
            ));
            if let Some(site) = site {
                content.push_str(&format!(" <small>({})</small>", escape_html(&site)));
            }

            let mut meta = Vec::new();
            if let Some(score) = score {
                meta.push(escape_html(&score));
            }
            if let Some(author) = author {
                meta.push(format!("by {}", escape_html(&author)));
            }
            if let Some(comments) = comments {
                meta.push(format!(
                    r#"<a href="{}">{}</a>"#,
                    escape_html(&comment_url),
                    escape_html(&comments)
                ));
            }
            if !meta.is_empty() {
                content.push_str("<br><small>");
                content.push_str(&meta.join(" · "));
                content.push_str("</small>");
            }
            content.push_str("</li>");
        }

        content.push_str("</ol>");
        if let Some(more) = dom::select_nodes(document, ".morelink[href]").into_iter().next()
            && let Some(href) = dom::attr(&more, "href").and_then(|href| Self::absolute_or_original(url, &href))
        {
            let label = dom::inner_text(&more);
            content.push_str(&format!(
                r#"<p><a href="{}">{}</a></p>"#,
                escape_html(&href),
                escape_html(label.trim())
            ));
        }
        content.push_str("</article></div>");

        let fragment = kuchiki::parse_html().one(format!("<html><body>{content}</body></html>"));
        let text_content = serialize::text_content(&dom::select_nodes(&fragment, "body"));
        let text_len = text_content.encode_utf16().count();

        Ok(Some(ExtractAttempt { metadata, content, text_content, text_len }))
    }

    fn hn_post_title(main_post: &NodeRef) -> Option<String> {
        dom::select_nodes(main_post, ".titleline")
            .into_iter()
            .next()
            .map(|node| decode_html_entities(&dom::inner_text(&node)))
            .filter(|title| !title.is_empty())
    }

    fn hn_story_url(main_post: &NodeRef) -> Option<String> {
        dom::select_nodes(main_post, ".titleline a[href]")
            .into_iter()
            .next()
            .and_then(|node| dom::attr(&node, "href"))
            .filter(|href| !href.is_empty())
    }

    fn hn_post_author(main_post: &NodeRef) -> Option<String> {
        dom::select_nodes(main_post, ".hnuser")
            .into_iter()
            .next()
            .map(|node| dom::inner_text(&node))
            .filter(|author| !author.is_empty())
    }

    fn hn_post_date(main_post: &NodeRef) -> Option<String> {
        dom::select_nodes(main_post, ".age")
            .into_iter()
            .next()
            .and_then(|node| dom::attr(&node, "title"))
            .and_then(|value| value.split_whitespace().next().map(str::to_string))
            .filter(|date| !date.is_empty())
    }

    fn hn_meta_line(main_post: &NodeRef, author: Option<&str>, published: Option<&str>) -> String {
        let score = dom::select_nodes(main_post, ".score")
            .into_iter()
            .next()
            .map(|node| dom::inner_text(&node))
            .unwrap_or_default();
        let mut parts = Vec::new();
        if !score.is_empty() {
            parts.push(score);
        }
        if let Some(author) = author.filter(|value| !value.is_empty()) {
            parts.push(format!("by {author}"));
        }
        if let Some(published) = published.filter(|value| !value.is_empty()) {
            parts.push(published.to_string());
        }
        parts.join(" · ")
    }

    fn hn_comments(document: &NodeRef) -> Result<Vec<HnComment>> {
        let mut roots: Vec<HnComment> = Vec::new();
        let mut path: Vec<usize> = Vec::new();

        for row in dom::select_nodes(document, "tr.comtr") {
            let Some(comment) = Self::hn_comment_from_row(&row)? else {
                continue;
            };
            let depth = Self::hn_comment_depth(&row);
            if depth == 0 {
                roots.push(comment);
                path.clear();
                path.push(roots.len() - 1);
                continue;
            }

            path.truncate(depth);
            if path.is_empty() {
                roots.push(comment);
                path.push(roots.len() - 1);
                continue;
            }

            let parent = Self::get_comment_mut(&mut roots, &path);
            parent.replies.push(comment);
            path.push(parent.replies.len() - 1);
        }

        Ok(roots)
    }

    fn hn_listing_comment_text(subtext: &NodeRef) -> String {
        dom::select_nodes(subtext, "a")
            .into_iter()
            .last()
            .map(|node| dom::inner_text(&node).replace('\u{a0}', " "))
            .unwrap_or_default()
    }

    fn next_element_sibling(node: &NodeRef, tag: &str) -> Option<NodeRef> {
        let mut sibling = node.next_sibling();
        while let Some(node) = sibling {
            if node.as_element().is_some() && dom::node_name(&node) == tag {
                return Some(node);
            }
            sibling = node.next_sibling();
        }
        None
    }

    fn absolute_or_original(base: &Url, href: &str) -> Option<String> {
        base.join(href)
            .ok()
            .map(|url| url.to_string())
            .or_else(|| (!href.trim().is_empty()).then(|| href.to_string()))
    }

    fn hn_comment_from_row(row: &NodeRef) -> Result<Option<HnComment>> {
        let Some(comment_text) = dom::select_nodes(row, ".commtext").into_iter().next() else {
            return Ok(None);
        };
        let id = dom::attr(row, "id").unwrap_or_default();
        let author = dom::select_nodes(row, ".hnuser")
            .into_iter()
            .next()
            .map(|node| dom::inner_text(&node))
            .filter(|author| !author.is_empty())
            .unwrap_or_else(|| "[deleted]".to_string());
        let date = dom::select_nodes(row, ".age")
            .into_iter()
            .next()
            .and_then(|node| dom::attr(&node, "title"))
            .and_then(|value| value.split_whitespace().next().map(str::to_string));
        let content_html = serialize::serialize_node(&comment_text)?;

        Ok(Some(HnComment {
            author,
            date,
            url: format!("https://news.ycombinator.com/item?id={id}"),
            content_html,
            replies: Vec::new(),
        }))
    }

    fn hn_comment_depth(row: &NodeRef) -> usize {
        dom::select_nodes(row, ".ind img")
            .into_iter()
            .next()
            .and_then(|node| dom::attr(&node, "width"))
            .and_then(|width| width.parse::<usize>().ok())
            .unwrap_or(0)
            / 40
    }

    fn get_comment_mut<'a>(comments: &'a mut [HnComment], path: &[usize]) -> &'a mut HnComment {
        let mut current = &mut comments[path[0]];
        for &index in &path[1..] {
            current = &mut current.replies[index];
        }
        current
    }
}
