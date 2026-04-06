use crate::{LectitoError, Result};
use regex::Regex;
use std::borrow::Cow;
use std::cell::RefCell;
use std::io::Read;
use std::rc::Rc;
use std::sync::OnceLock;
use url::Url;

/// Configuration for HTML preprocessing
#[derive(Debug, Clone)]
pub struct PreprocessConfig {
    /// Whether to remove script tags
    pub remove_scripts: bool,
    /// Whether to remove style tags
    pub remove_styles: bool,
    /// Whether to remove noscript tags
    pub remove_noscript: bool,
    /// Whether to remove iframe tags
    pub remove_iframes: bool,
    /// Whether to remove svg tags
    pub remove_svg: bool,
    /// Whether to remove canvas tags
    pub remove_canvas: bool,
    /// Whether to remove unlikely candidates
    pub remove_unlikely: bool,
    /// Whether to keep positive candidates even if they match unlikely patterns
    pub keep_positive: bool,
    /// Whether to remove hidden elements
    pub remove_hidden: bool,
    /// Whether to convert relative URLs to absolute
    pub convert_urls: bool,
    /// Base URL for converting relative URLs
    pub base_url: Option<Url>,
    /// Whether to resolve lazy-loaded images before scoring
    pub resolve_lazy_images: bool,
    /// Whether to preserve supported video embeds during preprocessing
    pub preserve_video_embeds: bool,
    /// Optional override regex for allowed video embed URLs
    pub video_embed_allowlist_regex: Option<String>,
}

impl Default for PreprocessConfig {
    fn default() -> Self {
        Self {
            remove_scripts: true,
            remove_styles: true,
            remove_noscript: true,
            remove_iframes: true,
            remove_svg: true,
            remove_canvas: true,
            remove_unlikely: true,
            keep_positive: true,
            remove_hidden: true,
            convert_urls: true,
            base_url: None,
            resolve_lazy_images: true,
            preserve_video_embeds: true,
            video_embed_allowlist_regex: None,
        }
    }
}

/// Preprocess HTML by removing unwanted elements and normalizing the document
pub fn preprocess_html(html: &str, config: &PreprocessConfig) -> String {
    let mut processed = rewrite_html(html, config);

    processed = remove_comments(&processed);
    processed = normalize_phrasing_content(&processed);

    if config.remove_unlikely {
        processed = remove_unlikely_candidates(&processed, config.keep_positive);
    }

    if config.remove_hidden {
        processed = remove_hidden_elements(&processed);
    }

    if config.convert_urls
        && let Some(base_url) = &config.base_url
    {
        processed = convert_relative_urls(&processed, base_url);
    }

    processed
}

/// Preprocess HTML from a reader using a streaming rewriter.
///
/// This avoids holding both the original and cleaned HTML in memory at once.
pub fn preprocess_reader<R: Read>(mut reader: R, config: &PreprocessConfig) -> Result<String> {
    let mut output = String::new();
    let mut rewriter = lol_html::HtmlRewriter::new(
        lol_html::Settings { element_content_handlers: streaming_handlers(config), ..Default::default() },
        |c: &[u8]| {
            output.push_str(&String::from_utf8_lossy(c));
        },
    );

    let mut buffer = [0u8; 8192];
    loop {
        let read_len = reader.read(&mut buffer).map_err(LectitoError::from)?;
        if read_len == 0 {
            break;
        }
        rewriter
            .write(&buffer[..read_len])
            .map_err(|e| LectitoError::HtmlParseError(e.to_string()))?;
    }

    rewriter
        .end()
        .map_err(|e| LectitoError::HtmlParseError(e.to_string()))?;

    let cleaned = if output.is_empty() { String::new() } else { output };
    let cleaned = remove_comments(&cleaned);
    Ok(normalize_phrasing_content(&cleaned))
}

fn streaming_handlers(
    config: &PreprocessConfig,
) -> Vec<(
    Cow<'static, lol_html::Selector>,
    lol_html::ElementContentHandlers<'static>,
)> {
    let mut handlers: Vec<(
        Cow<'static, lol_html::Selector>,
        lol_html::ElementContentHandlers<'static>,
    )> = Vec::new();
    let base_url = config.base_url.clone();
    let video_embed_allowlist = compiled_video_embed_allowlist(config.video_embed_allowlist_regex.as_deref());

    if config.remove_scripts {
        handlers.push(lol_html::element!("script", |el| {
            el.remove();
            Ok(())
        }));
    }
    if config.remove_styles {
        handlers.push(lol_html::element!("style", |el| {
            el.remove();
            Ok(())
        }));
    }
    if config.remove_noscript {
        if config.resolve_lazy_images {
            let noscript_buffers = Rc::new(RefCell::new(Vec::<String>::new()));
            let start_buffers = noscript_buffers.clone();
            handlers.push(lol_html::element!("noscript", move |el| {
                start_buffers.borrow_mut().push(String::new());
                el.remove_and_keep_content();
                Ok(())
            }));

            let text_buffers = noscript_buffers;
            let text_base_url = base_url.clone();
            handlers.push(lol_html::text!("noscript", move |text| {
                if let Some(buffer) = text_buffers.borrow_mut().last_mut() {
                    buffer.push_str(text.as_str());
                }

                text.remove();

                if text.last_in_text_node() {
                    let raw = text_buffers.borrow_mut().pop().unwrap_or_default();
                    if let Some(fragment) = extract_noscript_image_fragment(&raw, text_base_url.as_ref()) {
                        text.after(&fragment, lol_html::html_content::ContentType::Html);
                    }
                }

                Ok(())
            }));
        } else {
            handlers.push(lol_html::element!("noscript", |el| {
                el.remove();
                Ok(())
            }));
        }
    }
    if config.remove_iframes {
        let iframe_base = base_url.clone();
        let iframe_allowlist = video_embed_allowlist.clone();
        let preserve_video_embeds = config.preserve_video_embeds;
        handlers.push(lol_html::element!("iframe", move |el| {
            if preserve_video_embeds
                && embed_src_is_allowed(
                    el.get_attribute("src").as_deref(),
                    iframe_base.as_ref(),
                    iframe_allowlist.as_ref(),
                )
            {
                if let Some(base) = iframe_base.as_ref() {
                    absolutize_attr_url(el, "src", base);
                }
                return Ok(());
            }

            el.remove();
            Ok(())
        }));

        let embed_base = base_url.clone();
        let embed_allowlist = video_embed_allowlist.clone();
        let preserve_video_embeds = config.preserve_video_embeds;
        handlers.push(lol_html::element!("embed", move |el| {
            if preserve_video_embeds
                && embed_src_is_allowed(
                    el.get_attribute("src").as_deref(),
                    embed_base.as_ref(),
                    embed_allowlist.as_ref(),
                )
            {
                if let Some(base) = embed_base.as_ref() {
                    absolutize_attr_url(el, "src", base);
                }
                return Ok(());
            }

            el.remove();
            Ok(())
        }));

        let object_base = base_url.clone();
        let object_allowlist = video_embed_allowlist.clone();
        let preserve_video_embeds = config.preserve_video_embeds;
        handlers.push(lol_html::element!("object", move |el| {
            if preserve_video_embeds
                && embed_src_is_allowed(
                    el.get_attribute("data").as_deref(),
                    object_base.as_ref(),
                    object_allowlist.as_ref(),
                )
            {
                if let Some(base) = object_base.as_ref() {
                    absolutize_attr_url(el, "data", base);
                }
                return Ok(());
            }

            el.remove();
            Ok(())
        }));
    }
    if config.remove_svg {
        handlers.push(lol_html::element!("svg", |el| {
            el.remove();
            Ok(())
        }));
    }
    if config.remove_canvas {
        handlers.push(lol_html::element!("canvas", |el| {
            el.remove();
            Ok(())
        }));
    }

    if config.remove_unlikely {
        let unlikely_pattern = Regex::new(
            r"(?i)(banner|breadcrumbs?|combx|comment|community|disqus|extra|foot|header|menu|related|remark|rss|shoutbox|sidebar|sponsor|ad-break|agegate|pagination|pager|popup)",
        )
        .unwrap();
        let positive_pattern =
            Regex::new(r"(?i)(article|body|content|entry|hentry|h-entry|main|page|post|text|blog|story|tweet)")
                .unwrap();

        let keep_positive = config.keep_positive;
        handlers.push(lol_html::element!("*", move |el| {
            let tag_name = el.tag_name();
            if is_preserved_media_tag(tag_name.as_bytes()) {
                return Ok(());
            }

            if let Some(id) = el.get_attribute("id")
                && unlikely_pattern.is_match(&id)
                && (!keep_positive || !positive_pattern.is_match(&id))
            {
                el.remove_and_keep_content();
                return Ok(());
            }

            if let Some(class) = el.get_attribute("class") {
                for class_name in class.split_whitespace() {
                    if unlikely_pattern.is_match(class_name)
                        && (!keep_positive || !positive_pattern.is_match(class_name))
                    {
                        el.remove_and_keep_content();
                        return Ok(());
                    }
                }
            }

            Ok(())
        }));
    }

    if config.remove_hidden {
        handlers.push(lol_html::element!("*", move |el| {
            if hidden_reason(
                el.get_attribute("style").as_deref(),
                el.get_attribute("class").as_deref(),
            )
            .is_some()
            {
                el.remove();
            }
            Ok(())
        }));
    }

    if config.resolve_lazy_images || config.convert_urls {
        let img_base = base_url.clone();
        let resolve_lazy_images = config.resolve_lazy_images;
        let convert_img_urls = config.convert_urls;
        handlers.push(lol_html::element!("img", move |el| {
            if resolve_lazy_images {
                resolve_lazy_image_element(el);
                if el.removed() {
                    return Ok(());
                }
            }

            if convert_img_urls
                && let Some(base) = img_base.as_ref()
                && let Some(src) = el.get_attribute("src")
                && let Ok(absolute) = base.join(&src)
            {
                el.set_attribute("src", absolute.as_str()).ok();
            }

            Ok(())
        }));

        let source_base = base_url.clone();
        let convert_source_urls = config.convert_urls;
        if config.resolve_lazy_images || config.convert_urls {
            handlers.push(lol_html::element!("source", move |el| {
                resolve_lazy_source_element(el);
                if convert_source_urls && let Some(base) = source_base.as_ref() {
                    absolutize_attr_url(el, "src", base);
                }
                Ok(())
            }));
        }
    }

    if config.convert_urls
        && let Some(base_url) = base_url
    {
        let link_base = base_url.clone();
        handlers.push(lol_html::element!("a", move |el| {
            if let Some(href) = el.get_attribute("href")
                && let Ok(absolute) = link_base.join(&href)
            {
                el.set_attribute("href", absolute.as_str()).ok();
            }
            Ok(())
        }));

        let link_tag_base = base_url.clone();
        handlers.push(lol_html::element!("link", move |el| {
            if let Some(href) = el.get_attribute("href")
                && let Ok(absolute) = link_tag_base.join(&href)
            {
                el.set_attribute("href", absolute.as_str()).ok();
            }
            Ok(())
        }));

        let iframe_base = base_url.clone();
        handlers.push(lol_html::element!("iframe", move |el| {
            absolutize_attr_url(el, "src", &iframe_base);
            Ok(())
        }));

        let embed_base = base_url.clone();
        handlers.push(lol_html::element!("embed", move |el| {
            absolutize_attr_url(el, "src", &embed_base);
            Ok(())
        }));

        let object_base = base_url.clone();
        handlers.push(lol_html::element!("object", move |el| {
            absolutize_attr_url(el, "data", &object_base);
            Ok(())
        }));

        let video_base = base_url.clone();
        handlers.push(lol_html::element!("video", move |el| {
            absolutize_attr_url(el, "src", &video_base);
            absolutize_attr_url(el, "poster", &video_base);
            Ok(())
        }));

        let audio_base = base_url;
        handlers.push(lol_html::element!("audio", move |el| {
            absolutize_attr_url(el, "src", &audio_base);
            Ok(())
        }));
    }

    handlers.push(lol_html::element!("font", |el| {
        el.set_tag_name("span").ok();
        Ok(())
    }));

    handlers
}

fn rewrite_html(html: &str, config: &PreprocessConfig) -> String {
    let mut output = String::new();
    let mut rewriter = lol_html::HtmlRewriter::new(
        lol_html::Settings { element_content_handlers: streaming_handlers(config), ..Default::default() },
        |c: &[u8]| {
            output.push_str(&String::from_utf8_lossy(c));
        },
    );

    match rewriter.write(html.as_bytes()) {
        Ok(_) => {}
        Err(_) => return html.to_string(),
    }

    match rewriter.end() {
        Ok(_) => {}
        Err(_) => return html.to_string(),
    }

    if output.is_empty() { html.to_string() } else { output }
}

fn default_video_embed_allowlist_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r#"(?ix)
            ^
            (?:
                https?:)?
                //
                (?:
                    (?:www\.)?(?:youtube(?:-nocookie)?\.com|youtu\.be)/
                  | player\.vimeo\.com/
                  | (?:www\.)?dailymotion\.com/
                  | dai\.ly/
                  | (?:player|clips)\.twitch\.tv/
                  | (?:www\.)?bilibili\.com/
                  | player\.bilibili\.com/
                  | (?:www\.)?wikimedia\.org/
                  | commons\.wikimedia\.org/
                  | upload\.wikimedia\.org/
                )
            "#,
        )
        .unwrap()
    })
}

fn compiled_video_embed_allowlist(pattern: Option<&str>) -> Option<Regex> {
    match pattern {
        Some(pattern) => Regex::new(pattern).ok(),
        None => Some(default_video_embed_allowlist_regex().clone()),
    }
}

fn embed_src_is_allowed(src: Option<&str>, base_url: Option<&Url>, allowlist: Option<&Regex>) -> bool {
    let Some(src) = src.map(str::trim).filter(|src| !src.is_empty()) else {
        return false;
    };
    let Some(allowlist) = allowlist else {
        return false;
    };

    let candidate = normalize_embed_src(src, base_url).unwrap_or_else(|| src.to_string());
    allowlist.is_match(&candidate)
}

fn normalize_embed_src(src: &str, base_url: Option<&Url>) -> Option<String> {
    if src.starts_with("//") {
        return Some(format!("https:{src}"));
    }

    if src.starts_with("http://") || src.starts_with("https://") {
        return Some(src.to_string());
    }

    if src.starts_with('#') {
        return None;
    }

    base_url.and_then(|base| base.join(src).ok()).map(|url| url.to_string())
}

fn absolutize_attr_url(
    el: &mut lol_html::html_content::Element<'_, '_, impl lol_html::HandlerTypes>, attr: &str, base_url: &Url,
) {
    if let Some(value) = el.get_attribute(attr)
        && let Ok(absolute) = base_url.join(&value)
    {
        el.set_attribute(attr, absolute.as_str()).ok();
    }
}

fn is_preserved_media_tag(tag_name: &[u8]) -> bool {
    matches!(
        tag_name,
        b"audio" | b"embed" | b"iframe" | b"object" | b"source" | b"video"
    )
}

/// Remove script, style, noscript, iframe, svg, and canvas tags from HTML
#[cfg(test)]
fn remove_unwanted_tags(html: &str, config: &PreprocessConfig) -> String {
    rewrite_html(html, config)
}

/// Remove HTML comments from the document
fn remove_comments(html: &str) -> String {
    let re = Regex::new(r"(?s)<!--.*?-->").unwrap();
    re.replace_all(html, "").to_string()
}

fn extract_noscript_image_fragment(raw_html: &str, base_url: Option<&Url>) -> Option<String> {
    if !contains_image_markup(raw_html) {
        return None;
    }

    let fragment_config = PreprocessConfig {
        remove_scripts: true,
        remove_styles: true,
        remove_noscript: false,
        remove_iframes: true,
        remove_svg: false,
        remove_canvas: true,
        remove_unlikely: false,
        keep_positive: true,
        remove_hidden: false,
        convert_urls: base_url.is_some(),
        base_url: base_url.cloned(),
        resolve_lazy_images: true,
        preserve_video_embeds: true,
        video_embed_allowlist_regex: None,
    };

    let cleaned = remove_comments(&rewrite_html(raw_html.trim(), &fragment_config));
    contains_image_markup(&cleaned).then_some(cleaned)
}

fn contains_image_markup(html: &str) -> bool {
    static IMAGE_PATTERN: OnceLock<Regex> = OnceLock::new();
    IMAGE_PATTERN
        .get_or_init(|| Regex::new(r"(?is)<\s*(?:img|picture)\b").unwrap())
        .is_match(html)
}

fn resolve_lazy_image_element(el: &mut lol_html::html_content::Element<'_, '_, impl lol_html::HandlerTypes>) {
    let src = el.get_attribute("src");
    let src_is_placeholder = src.as_deref().is_some_and(is_placeholder_image_src);
    let src_is_lazy_fallback = src.as_deref().is_some_and(is_lazy_fallback_image_src);
    let promoted_src = promoted_attr_value(el, &["data-src", "data-original", "data-lazy-src"]);
    let promoted_srcset = promoted_attr_value(el, &["data-srcset"]);

    if let Some(srcset) = promoted_srcset
        && el
            .get_attribute("srcset")
            .is_none_or(|current| current.trim().is_empty())
    {
        el.set_attribute("srcset", &srcset).ok();
    }

    if let Some(promoted_src) = promoted_src
        && (src.is_none() || src_is_placeholder || src_is_lazy_fallback)
    {
        el.set_attribute("src", &promoted_src).ok();
    }

    let has_real_source = el
        .get_attribute("src")
        .is_some_and(|current| !current.trim().is_empty() && !is_placeholder_image_src(&current))
        || el
            .get_attribute("srcset")
            .is_some_and(|current| !current.trim().is_empty());

    if !has_real_source && src_is_placeholder {
        el.remove();
    }
}

fn resolve_lazy_source_element(el: &mut lol_html::html_content::Element<'_, '_, impl lol_html::HandlerTypes>) {
    if let Some(srcset) = promoted_attr_value(el, &["data-srcset", "data-src"])
        && el
            .get_attribute("srcset")
            .is_none_or(|current| current.trim().is_empty())
    {
        el.set_attribute("srcset", &srcset).ok();
    }
}

fn promoted_attr_value(
    el: &lol_html::html_content::Element<'_, '_, impl lol_html::HandlerTypes>, attrs: &[&str],
) -> Option<String> {
    attrs
        .iter()
        .find_map(|attr| el.get_attribute(attr))
        .and_then(|value| normalize_promoted_attr_value(&value))
}

fn normalize_promoted_attr_value(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || matches!(trimmed, "null" | "undefined") {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn is_placeholder_image_src(src: &str) -> bool {
    let trimmed = src.trim();
    if !trimmed.to_ascii_lowercase().starts_with("data:image/") {
        return false;
    }
    if trimmed.to_ascii_lowercase().starts_with("data:image/svg") {
        return false;
    }

    let Some((meta, payload)) = trimmed.split_once(',') else {
        return false;
    };

    let byte_len = if meta.to_ascii_lowercase().contains(";base64") {
        approximate_base64_decoded_len(payload)
    } else {
        payload.len()
    };

    byte_len < 133
}

fn is_lazy_fallback_image_src(src: &str) -> bool {
    static FALLBACK_PATTERN: OnceLock<Regex> = OnceLock::new();
    FALLBACK_PATTERN
        .get_or_init(|| {
            Regex::new(r"(?i)(?:lazyload[-_]?fallback|placeholder|spacer|blank|transparent|pixel|1x1)\.(?:gif|png|jpe?g|webp)(?:$|[?#])")
                .unwrap()
        })
        .is_match(src.trim())
}

fn approximate_base64_decoded_len(payload: &str) -> usize {
    let compact: String = payload.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    let padding = compact.chars().rev().take_while(|c| *c == '=').count();
    compact
        .len()
        .saturating_mul(3)
        .saturating_div(4)
        .saturating_sub(padding)
}

fn normalize_phrasing_content(html: &str) -> String {
    let mut normalized = html.to_string();

    for _ in 0..4 {
        let next = normalize_br_chain_paragraphs(&normalize_inline_only_divs(&normalized));
        if next == normalized {
            break;
        }
        normalized = next;
    }

    normalized
}

fn normalize_inline_only_divs(html: &str) -> String {
    static DIV_PATTERN: OnceLock<Regex> = OnceLock::new();
    let pattern = DIV_PATTERN.get_or_init(|| {
        Regex::new(&format!(
            r"(?is)<div(?P<attrs>[^>]*)>(?P<inner>{})</div>",
            inline_fragment_pattern()
        ))
        .unwrap()
    });

    pattern
        .replace_all(html, |caps: &regex::Captures<'_>| {
            let attrs = caps.name("attrs").map(|m| m.as_str()).unwrap_or("");
            let inner = caps.name("inner").map(|m| m.as_str()).unwrap_or("");
            if !should_convert_inline_div(inner) {
                return caps.get(0).unwrap().as_str().to_string();
            }
            wrap_phrasing_segments(inner, attrs)
        })
        .to_string()
}

fn normalize_br_chain_paragraphs(html: &str) -> String {
    static P_PATTERN: OnceLock<Regex> = OnceLock::new();
    let pattern = P_PATTERN.get_or_init(|| {
        Regex::new(&format!(
            r"(?is)<p(?P<attrs>[^>]*)>(?P<inner>{})</p>",
            inline_fragment_pattern()
        ))
        .unwrap()
    });

    pattern
        .replace_all(html, |caps: &regex::Captures<'_>| {
            let attrs = caps.name("attrs").map(|m| m.as_str()).unwrap_or("");
            let inner = caps.name("inner").map(|m| m.as_str()).unwrap_or("");

            if split_br_chain_segments(inner).len() <= 1 {
                return caps.get(0).unwrap().as_str().to_string();
            }

            wrap_phrasing_segments(inner, attrs)
        })
        .to_string()
}

fn inline_fragment_pattern() -> &'static str {
    r#"(?:(?:<!--.*?-->)|(?:[^<]+)|(?:</?(?:a|abbr|b|bdi|bdo|br|cite|code|data|del|dfn|em|font|i|img|ins|kbd|mark|q|rp|rt|ruby|s|samp|small|span|strike|strong|sub|sup|time|u|var|wbr)\b[^>]*>))*"#
}

fn should_convert_inline_div(inner: &str) -> bool {
    if split_br_chain_segments(inner).len() > 1 {
        return true;
    }

    has_top_level_text(inner)
}

fn has_top_level_text(inner: &str) -> bool {
    let mut depth = 0usize;
    let mut idx = 0usize;
    let bytes = inner.as_bytes();

    while idx < bytes.len() {
        if bytes[idx] == b'<' {
            let Some(end) = inner[idx..].find('>') else {
                break;
            };
            let tag = &inner[idx + 1..idx + end];
            let tag = tag.trim();

            if tag.starts_with('!') || tag.starts_with('?') {
                idx += end + 1;
                continue;
            }

            let closing = tag.starts_with('/');
            let tag_name = tag_name(tag.trim_start_matches('/'));
            let self_closing = tag.ends_with('/') || is_void_inline_tag(tag_name);

            if closing {
                depth = depth.saturating_sub(1);
            } else if !self_closing {
                depth += 1;
            }

            idx += end + 1;
            continue;
        }

        let Some(ch) = inner[idx..].chars().next() else {
            break;
        };
        if depth == 0 && !ch.is_whitespace() {
            return true;
        }
        idx += ch.len_utf8();
    }

    false
}

fn tag_name(tag: &str) -> &str {
    let end = tag
        .char_indices()
        .find(|(_, ch)| ch.is_whitespace() || *ch == '/')
        .map(|(idx, _)| idx)
        .unwrap_or(tag.len());
    &tag[..end]
}

fn is_void_inline_tag(tag_name: &str) -> bool {
    matches!(tag_name.to_ascii_lowercase().as_str(), "br" | "img" | "wbr")
}

fn wrap_phrasing_segments(inner: &str, attrs: &str) -> String {
    let segments = split_br_chain_segments(inner);
    if segments.len() <= 1 {
        return format!("<p{attrs}>{inner}</p>");
    }

    let mut output = String::new();
    for (idx, segment) in segments.into_iter().enumerate() {
        if idx == 0 {
            output.push_str(&format!("<p{attrs}>{segment}</p>"));
        } else {
            output.push_str(&format!("<p>{segment}</p>"));
        }
    }

    output
}

fn split_br_chain_segments(inner: &str) -> Vec<String> {
    static BR_CHAIN_PATTERN: OnceLock<Regex> = OnceLock::new();
    let pattern = BR_CHAIN_PATTERN.get_or_init(|| Regex::new(r"(?is)(?:\s*<br\b[^>]*>\s*){2,}").unwrap());

    if !pattern.is_match(inner) {
        return vec![inner.to_string()];
    }

    pattern
        .split(inner)
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(ToString::to_string)
        .collect()
}

/// Remove elements that match unlikely candidate patterns
fn remove_unlikely_candidates(html: &str, keep_positive: bool) -> String {
    let unlikely_pattern = Regex::new(
        r"(?i)(banner|breadcrumbs?|combx|comment|community|disqus|extra|foot|header|menu|related|remark|rss|shoutbox|sidebar|sponsor|ad-break|agegate|pagination|pager|popup)",
    ).unwrap();

    let positive_pattern =
        Regex::new(r"(?i)(article|body|content|entry|hentry|h-entry|main|page|post|text|blog|story|tweet)").unwrap();

    let mut output = String::new();
    let mut rewriter = lol_html::HtmlRewriter::new(
        lol_html::Settings {
            element_content_handlers: vec![lol_html::element!("*", |el| {
                if let Some(id) = el.get_attribute("id")
                    && unlikely_pattern.is_match(&id)
                    && (!keep_positive || !positive_pattern.is_match(&id))
                {
                    el.remove_and_keep_content();
                    return Ok(());
                }

                if let Some(class) = el.get_attribute("class") {
                    let classes: Vec<&str> = class.split_whitespace().collect();
                    for class_name in classes {
                        if unlikely_pattern.is_match(class_name)
                            && (!keep_positive || !positive_pattern.is_match(class_name))
                        {
                            el.remove_and_keep_content();
                            return Ok(());
                        }
                    }
                }

                Ok(())
            })],
            ..Default::default()
        },
        |c: &[u8]| {
            output.push_str(&String::from_utf8_lossy(c));
        },
    );

    match rewriter.write(html.as_bytes()) {
        Ok(_) => {}
        Err(_) => return html.to_string(),
    }

    match rewriter.end() {
        Ok(_) => {}
        Err(_) => return html.to_string(),
    }

    if output.is_empty() { html.to_string() } else { output }
}

/// Convert relative URLs to absolute URLs
pub fn convert_relative_urls(html: &str, base_url: &Url) -> String {
    let mut output = String::new();
    let mut rewriter = lol_html::HtmlRewriter::new(
        lol_html::Settings {
            element_content_handlers: vec![
                lol_html::element!("a", |el| {
                    if let Some(href) = el.get_attribute("href")
                        && let Ok(absolute) = base_url.join(&href)
                    {
                        el.set_attribute("href", absolute.as_str()).ok();
                    }
                    Ok(())
                }),
                lol_html::element!("img", |el| {
                    if let Some(src) = el.get_attribute("src")
                        && let Ok(absolute) = base_url.join(&src)
                    {
                        el.set_attribute("src", absolute.as_str()).ok();
                    }
                    Ok(())
                }),
                lol_html::element!("link", |el| {
                    if let Some(href) = el.get_attribute("href")
                        && let Ok(absolute) = base_url.join(&href)
                    {
                        el.set_attribute("href", absolute.as_str()).ok();
                    }
                    Ok(())
                }),
            ],
            ..Default::default()
        },
        |c: &[u8]| {
            output.push_str(&String::from_utf8_lossy(c));
        },
    );

    match rewriter.write(html.as_bytes()) {
        Ok(_) => {}
        Err(_) => return html.to_string(),
    }

    match rewriter.end() {
        Ok(_) => {}
        Err(_) => return html.to_string(),
    }

    if output.is_empty() { html.to_string() } else { output }
}

/// Remove elements with display:none or visibility:hidden styles
fn remove_hidden_elements(html: &str) -> String {
    let mut output = String::new();
    let mut rewriter = lol_html::HtmlRewriter::new(
        lol_html::Settings {
            element_content_handlers: vec![lol_html::element!("*", |el| {
                if hidden_reason(
                    el.get_attribute("style").as_deref(),
                    el.get_attribute("class").as_deref(),
                )
                .is_some()
                {
                    el.remove();
                    return Ok(());
                }
                Ok(())
            })],
            ..Default::default()
        },
        |c: &[u8]| {
            output.push_str(&String::from_utf8_lossy(c));
        },
    );

    match rewriter.write(html.as_bytes()) {
        Ok(_) => {}
        Err(_) => return html.to_string(),
    }

    match rewriter.end() {
        Ok(_) => {}
        Err(_) => return html.to_string(),
    }

    if output.is_empty() { html.to_string() } else { output }
}

pub(crate) fn hidden_reason(style: Option<&str>, class_name: Option<&str>) -> Option<String> {
    if let Some(style) = style
        && let Some(reason) = hidden_style_reason(style)
    {
        return Some(reason.to_string());
    }

    if let Some(class_name) = class_name
        && let Some(reason) = hidden_class_reason(class_name)
    {
        return Some(reason);
    }

    None
}

fn hidden_style_reason(style: &str) -> Option<&'static str> {
    static DISPLAY_NONE: OnceLock<Regex> = OnceLock::new();
    static VISIBILITY_HIDDEN: OnceLock<Regex> = OnceLock::new();
    static OPACITY_ZERO: OnceLock<Regex> = OnceLock::new();

    let display_none = DISPLAY_NONE
        .get_or_init(|| Regex::new(r"(?i)(?:^|;)\s*display\s*:\s*none(?:\s*!important)?\s*(?:;|$)").unwrap());
    if display_none.is_match(style) {
        return Some("display:none");
    }

    let visibility_hidden = VISIBILITY_HIDDEN
        .get_or_init(|| Regex::new(r"(?i)(?:^|;)\s*visibility\s*:\s*hidden(?:\s*!important)?\s*(?:;|$)").unwrap());
    if visibility_hidden.is_match(style) {
        return Some("visibility:hidden");
    }

    let opacity_zero = OPACITY_ZERO
        .get_or_init(|| Regex::new(r"(?i)(?:^|;)\s*opacity\s*:\s*0(?:\.0+)?(?:\s*!important)?\s*(?:;|$)").unwrap());
    if opacity_zero.is_match(style) {
        return Some("opacity:0");
    }

    None
}

fn hidden_class_reason(class_name: &str) -> Option<String> {
    static RESPONSIVE_HIDDEN: OnceLock<Regex> = OnceLock::new();

    let responsive_hidden = RESPONSIVE_HIDDEN.get_or_init(|| {
        Regex::new(
            r"(?ix)
            ^
            (?:
                (?:[a-z0-9_-]+:)+(?:hidden|invisible) |
                d-(?:xs|sm|md|lg|xl|xxl)-none |
                hidden-(?:xs|sm|md|lg|xl|xxl)(?:-(?:up|down))? |
                hide-for-[a-z-]+
            )
            $
            ",
        )
        .unwrap()
    });

    for token in class_name.split_whitespace() {
        let token_lower = token.to_ascii_lowercase();
        let is_hidden = matches!(
            token_lower.as_str(),
            "hidden"
                | "invisible"
                | "visually-hidden"
                | "visuallyhidden"
                | "screen-reader-only"
                | "sr-only"
                | "u-hidden"
                | "is-hidden"
                | "d-none"
        ) || responsive_hidden.is_match(&token_lower);

        if is_hidden {
            return Some(format!("class:{token}"));
        }
    }

    None
}

/// Normalize whitespace in HTML
#[cfg(test)]
fn normalize_whitespace(html: String) -> String {
    let re = Regex::new(r"\s+").unwrap();
    re.replace_all(&html, " ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_unwanted_tags() {
        let html = r#"
            <html>
                <head><script>alert('test');</script><style>body{color:red;}</style></head>
                <body>
                    <noscript>Enable JavaScript</noscript>
                    <iframe src="https://example.com"></iframe>
                    <svg><rect width="100" height="100"/></svg>
                    <canvas id="chart"></canvas>
                    <p>Content</p>
                </body>
            </html>
        "#;

        let config = PreprocessConfig {
            remove_scripts: true,
            remove_styles: true,
            remove_noscript: true,
            remove_iframes: true,
            remove_svg: true,
            remove_canvas: true,
            ..Default::default()
        };

        let result = remove_unwanted_tags(html, &config);
        assert!(!result.contains("<script"));
        assert!(!result.contains("<style"));
        assert!(!result.contains("<noscript"));
        assert!(!result.contains("<iframe"));
        assert!(!result.contains("<svg"));
        assert!(!result.contains("<canvas"));
        assert!(result.contains("<p>Content</p>"));

        assert!(!result.contains("alert"), "Script content should be removed");
        assert!(!result.contains("color:red"), "Style content should be removed");
        assert!(
            !result.contains("Enable JavaScript"),
            "Noscript content should be removed"
        );
        assert!(!result.contains("example.com"), "Iframe src should be removed");
        assert!(!result.contains("rect"), "SVG content should be removed");
        assert!(!result.contains("chart"), "Canvas id should be removed");
    }

    #[test]
    fn test_preserve_allowed_video_iframe() {
        let html = r#"
            <html><body>
                <figure class="video-embed">
                    <iframe src="https://www.youtube.com/embed/abc123"></iframe>
                </figure>
            </body></html>
        "#;

        let result = preprocess_html(html, &PreprocessConfig::default());
        assert!(result.contains("<iframe"));
        assert!(result.contains("youtube.com/embed/abc123"));
    }

    #[test]
    fn test_remove_disallowed_video_iframe() {
        let html = r#"
            <html><body>
                <iframe src="https://ads.example.com/embed/abc123"></iframe>
            </body></html>
        "#;

        let result = preprocess_html(html, &PreprocessConfig::default());
        assert!(!result.contains("<iframe"));
        assert!(!result.contains("ads.example.com"));
    }

    #[test]
    fn test_disable_video_embed_preservation() {
        let html = r#"
            <html><body>
                <iframe src="https://player.vimeo.com/video/42"></iframe>
            </body></html>
        "#;

        let config = PreprocessConfig { preserve_video_embeds: false, ..Default::default() };
        let result = preprocess_html(html, &config);
        assert!(!result.contains("<iframe"));
    }

    #[test]
    fn test_remove_comments() {
        let html = r#"
            <html>
                <body>
                    <!-- This is a comment -->
                    <p>Visible content</p>
                    <!-- Another comment -->
                </body>
            </html>
        "#;

        let result = remove_comments(html);
        assert!(!result.contains("<!--"));
        assert!(result.contains("Visible content"));
    }

    #[test]
    fn test_remove_unlikely_candidates() {
        let html = r#"
            <html>
                <body>
                    <div id="sidebar">Sidebar content</div>
                    <div id="main-content">Main content</div>
                    <div class="banner-ad">Ad</div>
                    <div class="article">Article content</div>
                </body>
            </html>
        "#;

        let result = remove_unlikely_candidates(html, true);
        assert!(!result.contains("sidebar"));
        assert!(!result.contains("banner-ad"));
        assert!(result.contains("main-content"));
        assert!(result.contains("article"));
    }

    #[test]
    fn test_convert_relative_urls() {
        let base = Url::parse("https://example.com/blog/").unwrap();
        let html = r#"
            <html>
                <body>
                    <a href="/about">About</a>
                    <a href="post.html">Post</a>
                    <img src="image.jpg" />
                </body>
            </html>
        "#;

        let result = convert_relative_urls(html, &base);
        assert!(result.contains("href=\"https://example.com/about\""));
        assert!(result.contains("href=\"https://example.com/blog/post.html\""));
        assert!(result.contains("src=\"https://example.com/blog/image.jpg\""));
    }

    #[test]
    fn test_remove_hidden_elements() {
        let html = r#"
            <html>
                <body>
                    <div style="display:none">Hidden content</div>
                    <div style="visibility:hidden">Invisible content</div>
                    <div style="opacity:0">Transparent content</div>
                    <div class="hidden">Tailwind hidden</div>
                    <div class="lg:hidden">Responsive hidden</div>
                    <div class="d-none">Bootstrap hidden</div>
                    <div>Visible content</div>
                </body>
            </html>
        "#;

        let result = remove_hidden_elements(html);
        assert!(!result.contains("Hidden content"));
        assert!(!result.contains("Invisible content"));
        assert!(!result.contains("Transparent content"));
        assert!(!result.contains("Tailwind hidden"));
        assert!(!result.contains("Responsive hidden"));
        assert!(!result.contains("Bootstrap hidden"));
        assert!(result.contains("Visible content"));
    }

    #[test]
    fn test_normalize_whitespace() {
        let html = "<html><body>    Multiple   spaces\t\t\n\nhere</body></html>";
        let result = normalize_whitespace(html.to_string());
        let spaces_before = html.matches(|c: char| c.is_whitespace()).count();
        let spaces_after = result.matches(' ').count();
        assert!(spaces_after < spaces_before);
    }

    #[test]
    fn test_preprocess_full_pipeline() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <script>console.log('test');</script>
                <style>.hidden{display:none;}</style>
                <!-- Comment -->
            </head>
            <body>
                <div id="sidebar" class="menu">
                    <p>Sidebar</p>
                </div>
                <div id="main" class="article">
                    <a href="/post">Link</a>
                    <p style="display:none">Hidden</p>
                    <p>Content</p>
                </div>
            </body>
            </html>
        "#;

        let base = Url::parse("https://example.com").unwrap();
        let config = PreprocessConfig { base_url: Some(base), ..Default::default() };

        let result = preprocess_html(html, &config);

        assert!(!result.contains("<script"));
        assert!(!result.contains("<style"));
        assert!(!result.contains("<!--"));
        assert!(!result.contains("sidebar"));
        assert!(!result.contains("Hidden"));
        assert!(result.contains("main"));
        assert!(result.contains("href=\"https://example.com/post\""));
        assert!(result.contains("Content"));
    }

    #[test]
    fn test_resolve_lazy_image_attributes() {
        let html = r#"
            <html>
                <body>
                    <img src="data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///ywAAAAAAQABAAACAUwAOw==" data-src="/hero.jpg" data-srcset="/hero.jpg 1200w" />
                    <picture>
                        <source data-srcset="/hero.webp 1200w" />
                    </picture>
                </body>
            </html>
        "#;

        let base = Url::parse("https://example.com/articles/").unwrap();
        let result = preprocess_html(html, &PreprocessConfig { base_url: Some(base), ..Default::default() });

        assert!(result.contains("src=\"https://example.com/hero.jpg\""));
        assert!(result.contains("srcset=\"/hero.jpg 1200w\""));
        assert!(result.contains("srcset=\"/hero.webp 1200w\""));
        assert!(!result.contains("R0lGODlhAQABAIAAAAAAAP///ywAAAAAAQABAAACAUwAOw=="));
    }

    #[test]
    fn test_unwrap_noscript_image_replacement() {
        let html = r#"
            <html>
                <body>
                    <img src="data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///ywAAAAAAQABAAACAUwAOw==" />
                    <noscript><img src="/fullsize.jpg" alt="Full size image"></noscript>
                </body>
            </html>
        "#;

        let base = Url::parse("https://example.com/").unwrap();
        let result = preprocess_html(html, &PreprocessConfig { base_url: Some(base), ..Default::default() });

        assert!(!result.contains("<noscript"));
        assert!(result.contains("src=\"https://example.com/fullsize.jpg\""));
        assert!(result.contains("alt=\"Full size image\""));
        assert!(!result.contains("R0lGODlhAQABAIAAAAAAAP///ywAAAAAAQABAAACAUwAOw=="));
    }

    #[test]
    fn test_normalize_br_chains_and_inline_divs() {
        let html = r#"
            <html>
                <body>
                    <div class="story">Alpha<br><br>Beta <font color="red">Gamma</font></div>
                </body>
            </html>
        "#;

        let result = normalize_whitespace(preprocess_html(html, &PreprocessConfig::default()));

        assert!(result.contains(r#"<p class="story">Alpha</p><p>Beta <span color="red">Gamma</span></p>"#));
        assert!(!result.contains("<font"));
    }

    #[test]
    fn test_inline_div_with_block_child_stays_div() {
        let html = r#"
            <html>
                <body>
                    <div class="wrapper"><span>Intro</span><p>Body</p></div>
                </body>
            </html>
        "#;

        let result = preprocess_html(html, &PreprocessConfig::default());

        assert!(result.contains(r#"<div class="wrapper"><span>Intro</span><p>Body</p></div>"#));
    }
}
