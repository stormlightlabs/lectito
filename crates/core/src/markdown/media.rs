use comrak::{escape_commonmark_inline, escape_commonmark_link_destination};
use kuchiki::NodeRef;
use url::Url;

use super::{RenderContext, block, inline_children};
use crate::{dom, patterns};

#[derive(Debug, Clone)]
struct ImageCandidate {
    url: String,
    width: Option<u32>,
    density: Option<f32>,
    order: usize,
}

pub(super) fn render_image(node: &NodeRef) -> String {
    render_image_from_candidates(node, image_candidates(node, 0))
}

pub(super) fn render_picture(node: &NodeRef) -> String {
    let img = dom::select_nodes(node, "img").into_iter().next();
    let mut candidates = Vec::new();
    let mut order = 0;
    for source in dom::select_nodes(node, "source") {
        candidates.extend(srcset_candidates(&source, order));
        order = candidates.len();
    }
    if let Some(img) = img.as_ref() {
        candidates.extend(image_candidates(img, order));
    }
    render_image_from_candidates(img.as_ref().unwrap_or(node), candidates)
}

pub(super) fn render_figure(node: &NodeRef, ctx: RenderContext) -> Option<String> {
    let media = dom::select_nodes(node, "picture, img, iframe, video, audio, object, embed, blockquote")
        .into_iter()
        .filter(|candidate| dom::node_id(candidate) != dom::node_id(node))
        .filter(|candidate| {
            dom::node_name(candidate) != "img"
                || !candidate.ancestors().any(|ancestor| {
                    dom::node_id(&ancestor) != dom::node_id(node) && dom::node_name(&ancestor) == "picture"
                })
        })
        .filter_map(|candidate| match dom::node_name(&candidate).as_str() {
            "picture" => Some(render_picture(&candidate)),
            "img" => Some(render_image(&candidate)),
            "blockquote" | "iframe" | "video" | "audio" | "object" | "embed" => render_embed(&candidate),
            _ => None,
        })
        .filter(|markdown| !markdown.is_empty())
        .collect::<Vec<String>>();
    if media.len() != 1 || !patterns::normalize_spaces(&non_caption_text(node)).trim().is_empty() {
        return None;
    }

    let mut output = media.into_iter().next().unwrap();
    if let Some(caption) = dom::select_nodes(node, "figcaption")
        .into_iter()
        .map(|caption| inline_children(&caption, ctx))
        .find(|caption| !caption.is_empty())
    {
        output.push_str("\n\n");
        output.push_str(&caption);
    }

    Some(block(output))
}

pub(super) fn render_embed(node: &NodeRef) -> Option<String> {
    let mut embed_url = None;
    if dom::node_name(node) == "blockquote" {
        let class = dom::attr(node, "class").unwrap_or_default();
        if !class.contains("twitter-tweet") && !class.contains("x-tweet") {
            embed_url = None
        } else {
            embed_url = dom::select_nodes(node, "a")
                .into_iter()
                .filter_map(|link| dom::attr(&link, "href"))
                .find_map(|href| normalize_embed_url(&href))
        }
    } else {
        for attr in ["src", "data-src", "data", "href"] {
            if let Some(url) = dom::attr(node, attr)
                && let Some(embed) = normalize_embed_url(&url)
            {
                embed_url = Some(embed)
            }
        }
    }

    embed_url.map(|url| format!("![]({})", escape_commonmark_link_destination(&url)))
}

fn render_image_from_candidates(node: &NodeRef, candidates: Vec<ImageCandidate>) -> String {
    let Some(src) = best_image_url(candidates) else {
        return String::new();
    };

    let alt = dom::attr(node, "alt").unwrap_or_default();
    let title = dom::attr(node, "title")
        .filter(|title| !title.trim().is_empty())
        .map(|title| format!(" \"{}\"", title.replace('\\', "\\\\").replace('"', "\\\"")))
        .unwrap_or_default();

    format!(
        "![{}]({}{})",
        escape_commonmark_inline(&alt),
        escape_commonmark_link_destination(&src),
        title
    )
}

fn image_candidates(node: &NodeRef, start_order: usize) -> Vec<ImageCandidate> {
    let mut candidates = srcset_candidates(node, start_order);
    let mut order = start_order + candidates.len();
    for attr in ["data-src", "data-original", "data-lazy-src", "data-url", "src"] {
        if let Some(url) = dom::attr(node, attr).filter(|url| !url.trim().is_empty()) {
            candidates.push(ImageCandidate { url, width: None, density: None, order });
            order += 1;
        }
    }
    candidates
}

fn srcset_candidates(node: &NodeRef, start_order: usize) -> Vec<ImageCandidate> {
    ["data-srcset", "srcset"]
        .into_iter()
        .filter_map(|attr| dom::attr(node, attr))
        .flat_map(|srcset| parse_srcset(&srcset))
        .enumerate()
        .map(|(index, mut candidate)| {
            candidate.order = start_order + index;
            candidate
        })
        .collect()
}

fn parse_srcset(srcset: &str) -> Vec<ImageCandidate> {
    let parts: Vec<_> = srcset.split(',').collect();
    let mut candidates = Vec::new();
    let mut start = 0;
    let mut current = String::new();

    for (index, part) in parts.iter().enumerate() {
        if !current.is_empty() {
            current.push(',');
        }
        current.push_str(part);

        if current
            .split_whitespace()
            .last()
            .is_some_and(|value| parse_width_descriptor(value).is_some() || parse_density_descriptor(value).is_some())
        {
            if let Some(candidate) = srcset.get(start..start + current.len()) {
                candidates.push(candidate.trim());
            }
            start += current.len() + usize::from(index + 1 < parts.len());
            current.clear();
        }
    }
    if !current.trim().is_empty()
        && let Some(candidate) = srcset.get(start..)
    {
        candidates.push(candidate.trim());
    }

    candidates
        .into_iter()
        .filter(|candidate| !candidate.is_empty())
        .filter_map(|candidate| {
            let mut parts = candidate.split_whitespace();
            let url = parts.next()?.to_string();
            if url.is_empty() {
                return None;
            }
            let descriptor = parts.last();
            let width = descriptor.and_then(parse_width_descriptor);
            let density = descriptor.and_then(parse_density_descriptor);
            Some(ImageCandidate { url, width, density, order: 0 })
        })
        .collect()
}

fn parse_width_descriptor(value: &str) -> Option<u32> {
    value.strip_suffix('w')?.parse::<u32>().ok()
}

fn parse_density_descriptor(value: &str) -> Option<f32> {
    value.strip_suffix('x')?.parse::<f32>().ok()
}

fn best_image_url(candidates: Vec<ImageCandidate>) -> Option<String> {
    candidates
        .into_iter()
        .filter(|candidate| {
            let trimmed = candidate.url.trim().to_ascii_lowercase();
            !(trimmed.is_empty()
                || trimmed.starts_with("data:")
                || trimmed == "#"
                || trimmed == "about:blank"
                || trimmed.contains("placeholder"))
        })
        .max_by(|a, b| {
            image_score(a)
                .total_cmp(&image_score(b))
                .then_with(|| b.order.cmp(&a.order))
        })
        .map(|candidate| candidate.url)
}

fn image_score(candidate: &ImageCandidate) -> f32 {
    if let Some(width) = candidate.width {
        width as f32
    } else if let Some(density) = candidate.density {
        density * 1000.0
    } else {
        0.0
    }
}

fn non_caption_text(node: &NodeRef) -> String {
    match node.as_text() {
        Some(text) => text.borrow().to_string(),
        None => match dom::node_name(node).as_str() {
            "figcaption" | "picture" | "img" | "iframe" | "video" | "audio" | "object" | "embed" => String::new(),
            _ => node
                .children()
                .map(|child| non_caption_text(&child))
                .collect::<Vec<_>>()
                .join(" "),
        },
    }
}

fn normalize_embed_url(url: &str) -> Option<String> {
    let parsed = Url::parse(url).ok()?;
    let host = parsed.host_str()?.trim_start_matches("www.");
    if host == "youtu.be" {
        let id = parsed.path_segments()?.next()?.trim();
        if !id.is_empty() {
            return Some(format!("https://www.youtube.com/watch?v={id}"));
        }
    }
    if host.ends_with("youtube.com") || host.ends_with("youtube-nocookie.com") {
        if parsed.path() == "/watch"
            && let Some(id) = parsed
                .query_pairs()
                .find(|(name, _)| name == "v")
                .map(|(_, value)| value)
        {
            return Some(format!("https://www.youtube.com/watch?v={id}"));
        }
        for prefix in ["/embed/", "/shorts/"] {
            if let Some(id) = parsed
                .path()
                .strip_prefix(prefix)
                .and_then(|path| path.split('/').next())
                && !id.is_empty()
            {
                return Some(format!("https://www.youtube.com/watch?v={id}"));
            }
        }
    }
    if (host.ends_with("twitter.com") || host.ends_with("x.com")) && parsed.path().contains("/status/") {
        return Some(url.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_srcset_urls_with_commas() {
        let candidates = super::parse_srcset(
            "https://cdn.example.com/image,w_400.jpg 400w, https://cdn.example.com/image,w_1600.jpg 1600w",
        );

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].url, "https://cdn.example.com/image,w_400.jpg");
        assert_eq!(candidates[1].url, "https://cdn.example.com/image,w_1600.jpg");
        assert_eq!(
            super::best_image_url(candidates).as_deref(),
            Some("https://cdn.example.com/image,w_1600.jpg")
        );
    }

    #[test]
    fn parses_srcset_without_spaces_after_commas() {
        let candidates = super::parse_srcset("small.png 36w,medium.png 480w,large.png 2880w");

        assert_eq!(candidates.len(), 3);
        assert_eq!(super::best_image_url(candidates).as_deref(), Some("large.png"));
    }
}
