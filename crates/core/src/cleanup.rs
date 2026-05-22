use kuchiki::NodeRef;
use once_cell::sync::Lazy;
use regex::Regex;
use url::Url;

use super::config::{ExtractFlags, ReadabilityOptions};
use super::metadata::Metadata;
use super::patterns::{
    AD_OR_LOADING_WORDS, COMMA, DEFAULT_CLASSES_TO_PRESERVE, DEPRECATED_SIZE_ATTRIBUTE_ELEMS,
    PRESENTATIONAL_ATTRIBUTES, SHARE_ELEMENTS,
};
use super::scoring::{class_weight, link_density};
use super::{dom, markdown};

static LAZY_IMAGE_URL: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^\s*\S+\.(jpg|jpeg|png|webp)(\?\S*)?\s*$").expect("valid image url regex"));

static LAZY_IMAGE_SRCSET: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\.(jpg|jpeg|png|webp)\S*\s+\d").expect("valid image srcset regex"));

static TRAILING_CHROME_ATTRS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?ix)
        \b(
            after[-_]?post|bottom[-_]?of[-_]?article|comment(s|ary)?|comment[-_]?thread|court[-_]?case|
            discussion|disqus|finance|follow[-_]?up|job(s)?|keep[-_]?reading|
            mortgage|most[-_]?popular|most[-_]?read|most[-_]?viewed|newsletter|next[-_]?article|next[-_]?up|
            onward[-_]?journey|outbrain|partner[-_]?offer|popular|promo|
            read[-_]?also|read[-_]?more|read[-_]?next|recommend(ed|ation|ations)?|
            recirc|related|signup|sign[-_]?up|sponsor(ed)?|subscribe|
            subscription|taboola|widget|yarpp
        )\b",
    )
    .expect("valid trailing chrome attribute regex")
});

static TRAILING_CHROME_TEXT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)^\s*(comments?|join the (conversation|discussion)|related( articles| posts| stories)?|also in\b|court case:|affiliate:|explore press release|more (from|in|on)|recommended|most (popular|read|viewed)|next article|read (also|more|next)|sponsored|partner offers?|(the )?\w*\s*newsletter|sign up|subscribe|jobs?|mortgage|finance)",
    )
    .expect("valid trailing chrome text regex")
});

static FOOTNOTE_REFERENCE_ATTRS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(footnotes?|endnotes?|references?|bibliography|citations?)\b")
        .expect("valid footnote reference attribute regex")
});

static FOOTNOTE_REFERENCE_TEXT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^\s*(footnotes?|notes|references|bibliography|citations?)\s*$")
        .expect("valid footnote reference text regex")
});

static LEADING_DATE_TEXT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?ix)^\s*
        (
            (jan|feb|mar|apr|may|jun|jul|aug|sep|sept|oct|nov|dec)[a-z]*\.?\s+\d{1,2},?\s+\d{4}
            |
            \d{1,2}\s+(jan|feb|mar|apr|may|jun|jul|aug|sep|sept|oct|nov|dec)[a-z]*\.?\s+\d{4}
            |
            \d{4}[-/]\d{1,2}[-/]\d{1,2}
        )
        \s*$",
    )
    .expect("valid leading date text regex")
});

pub(crate) fn cleanup_article(
    nodes: &[NodeRef], options: &ReadabilityOptions, flags: ExtractFlags, base_url: Option<&Url>, metadata: &Metadata,
) {
    for node in nodes {
        clean_styles(node);
        clean_unsafe_attrs(node);
        fix_lazy_images(node);
        dom::remove_matching(
            node,
            "script, style, noscript, base, form, fieldset, footer, link, aside",
        );
        clean_embeds(node);
        remove_share_nodes(node);
        remove_trailing_page_chrome(node);
        clean_headers(node, metadata.title.as_deref(), flags);
        clean_leading_article_metadata(node, metadata);
        markdown::code::normalize_code_markup(node);
        if flags.clean_conditionally {
            clean_conditionally(node, options, flags);
        }
        remove_empty_blocks(node);
        fix_relative_urls(node, base_url);
        if !options.keep_classes {
            clean_classes(node, options);
        }
    }
}

pub(crate) fn remove_trailing_chrome_roots(roots: Vec<NodeRef>) -> Vec<NodeRef> {
    let mut retained = Vec::with_capacity(roots.len());
    let mut trimming = true;

    for root in roots.into_iter().rev() {
        if trimming && is_trailing_page_chrome(&root) {
            continue;
        }
        if is_footnote_or_reference_block(&root) || has_meaningful_article_content(&root) {
            trimming = false;
        }
        retained.push(root);
    }

    retained.reverse();
    retained
}

fn clean_unsafe_attrs(node: &NodeRef) {
    let attrs = dom::attrs(node);
    for (name, value) in attrs {
        let lower_name = name.to_ascii_lowercase();
        let lower_value = value.trim_start().to_ascii_lowercase();
        if lower_name.starts_with("on")
            || lower_name == "srcdoc"
            || (matches!(lower_name.as_str(), "href" | "src") && unsafe_url_value(&lower_value))
        {
            dom::remove_attr(node, &name);
        }
    }

    for child in node.children() {
        clean_unsafe_attrs(&child);
    }
}

fn unsafe_url_value(value: &str) -> bool {
    value.starts_with("javascript:") || value.starts_with("data:text/html")
}

fn clean_embeds(root: &NodeRef) {
    for node in dom::select_nodes(root, "object, embed, iframe") {
        let keep = dom::attrs(&node).values().any(|value| allowed_video(value));
        if !keep {
            node.detach();
        }
    }
}

fn allowed_video(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    [
        "youtube.com",
        "youtube-nocookie.com",
        "player.vimeo.com",
        "dailymotion.com",
        "player.twitch.tv",
        "archive.org",
        "upload.wikimedia.org",
    ]
    .iter()
    .any(|needle| value.contains(needle))
}

fn clean_styles(node: &NodeRef) {
    if let Some(element) = node.as_element() {
        let tag = element.name.local.to_string();
        let mut attrs = element.attributes.borrow_mut();
        for attr in PRESENTATIONAL_ATTRIBUTES {
            attrs.remove(*attr);
        }
        if DEPRECATED_SIZE_ATTRIBUTE_ELEMS.contains(&tag.as_str()) {
            attrs.remove("width");
            attrs.remove("height");
        }
    }

    for child in node.children() {
        clean_styles(&child);
    }
}

fn fix_lazy_images(root: &NodeRef) {
    for node in dom::select_nodes(root, "img, picture, figure") {
        let tag = dom::node_name(&node);
        let attrs_snapshot = dom::attrs(&node);
        if attrs_snapshot.contains_key("src")
            && !attrs_snapshot
                .get("class")
                .is_some_and(|class| class.to_lowercase().contains("lazy"))
        {
            continue;
        }

        for (name, value) in attrs_snapshot {
            if matches!(name.as_str(), "src" | "srcset" | "alt") {
                continue;
            }

            let copy_to = if LAZY_IMAGE_SRCSET.is_match(&value) {
                Some("srcset")
            } else if LAZY_IMAGE_URL.is_match(&value) {
                Some("src")
            } else {
                None
            };

            if let Some(copy_to) = copy_to
                && (tag == "img" || tag == "picture")
            {
                dom::set_attr(&node, copy_to, &value);
            }
        }
    }
}

fn remove_share_nodes(root: &NodeRef) {
    for node in dom::select_nodes(root, "*") {
        if SHARE_ELEMENTS.is_match(&dom::class_id_string(&node)) && dom::inner_text(&node).chars().count() < 500 {
            node.detach();
        }
    }
}

fn remove_trailing_page_chrome(root: &NodeRef) {
    let mut trimming = true;
    for child in root
        .children()
        .filter(|node| node.as_element().is_some())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        if !trimming {
            break;
        }
        if is_trailing_page_chrome(&child) {
            child.detach();
        } else if is_footnote_or_reference_block(&child) || has_meaningful_article_content(&child) {
            trimming = false;
        }
    }

    for child in root.children().filter(|node| node.as_element().is_some()) {
        if !is_footnote_or_reference_block(&child) {
            remove_trailing_page_chrome(&child);
        }
    }
}

fn is_trailing_page_chrome(node: &NodeRef) -> bool {
    if is_footnote_or_reference_block(node) {
        return false;
    }

    let tag = dom::node_name(node);
    if matches!(tag.as_str(), "aside" | "footer" | "form" | "nav") {
        return true;
    }

    let attrs = trailing_signal_attrs(node);
    if TRAILING_CHROME_ATTRS.is_match(&attrs) {
        return true;
    }

    let text = dom::inner_text(node);
    let text_len = text.chars().count();
    if text_len == 0 {
        return false;
    }
    if text_len < 500 && text.to_ascii_lowercase().contains("explore press release") {
        return true;
    }
    if text_len < 800 && TRAILING_CHROME_TEXT.is_match(&text) {
        return true;
    }

    let link_count = dom::select_nodes(node, "a").len();
    if link_count >= 3 && link_density(node) > 0.45 && text_len < 1500 {
        return true;
    }

    let input_count = dom::select_nodes(node, "input, button, select, textarea").len();
    input_count > 0 && text_len < 700 && looks_like_signup_text(&text)
}

fn is_footnote_or_reference_block(node: &NodeRef) -> bool {
    let attrs = trailing_signal_attrs(node);
    if FOOTNOTE_REFERENCE_ATTRS.is_match(&attrs) {
        return true;
    }

    if dom::attr(node, "role")
        .is_some_and(|role| matches!(role.to_ascii_lowercase().as_str(), "doc-footnotes" | "doc-endnotes"))
    {
        return true;
    }

    dom::select_nodes(node, "h1, h2, h3, h4, h5, h6")
        .first()
        .is_some_and(|heading| FOOTNOTE_REFERENCE_TEXT.is_match(&dom::inner_text(heading)))
}

fn has_meaningful_article_content(node: &NodeRef) -> bool {
    let tag = dom::node_name(node);
    if matches!(tag.as_str(), "p" | "pre" | "blockquote" | "table" | "figure") {
        return !dom::inner_text(node).is_empty() || !dom::select_nodes(node, "img, picture, video, audio").is_empty();
    }

    let text = dom::inner_text(node);
    if text.chars().count() >= 80 && link_density(node) < 0.5 {
        return true;
    }

    !dom::select_nodes(node, "p, pre, blockquote, table, figure, img, picture, video, audio").is_empty()
}

fn trailing_signal_attrs(node: &NodeRef) -> String {
    let attrs = dom::attrs(node);
    [
        attrs.get("class").map(String::as_str).unwrap_or_default(),
        attrs.get("id").map(String::as_str).unwrap_or_default(),
        attrs.get("role").map(String::as_str).unwrap_or_default(),
        attrs.get("data-component").map(String::as_str).unwrap_or_default(),
        attrs.get("data-testid").map(String::as_str).unwrap_or_default(),
        attrs.get("data-test").map(String::as_str).unwrap_or_default(),
        attrs.get("aria-label").map(String::as_str).unwrap_or_default(),
    ]
    .join(" ")
    .to_ascii_lowercase()
}

fn looks_like_signup_text(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    ["newsletter", "sign up", "signup", "subscribe", "email", "offer"]
        .iter()
        .any(|needle| lower.contains(needle))
}

fn clean_headers(root: &NodeRef, article_title: Option<&str>, flags: ExtractFlags) {
    for node in dom::select_nodes(root, "h1, h2") {
        let low_weight = class_weight(&node, flags) < 0;
        let duplicates_title = article_title
            .map(|title| text_similarity(title, &dom::inner_text(&node)) > 0.75)
            .unwrap_or(false);
        if low_weight || duplicates_title {
            node.detach();
        }
    }
}

fn clean_leading_article_metadata(root: &NodeRef, metadata: &Metadata) {
    for node in dom::select_nodes(root, "header, hgroup") {
        if dom::node_id(&node) == dom::node_id(root) {
            continue;
        }
        if looks_like_article_header(&node, metadata) {
            node.detach();
        }
    }

    for node in dom::select_nodes(root, "time") {
        if matches_metadata_value(&dom::inner_text(&node), metadata.published_time.as_deref())
            || dom::attr(&node, "datetime").is_some()
        {
            node.detach();
        }
    }
    for node in dom::select_nodes(root, "p, div, span") {
        if dom::node_id(&node) != dom::node_id(root) && is_leading_date_node(&node) {
            node.detach();
        }
    }

    trim_leading_metadata_siblings(root, metadata);
    for child in root.children().filter(|node| node.as_element().is_some()) {
        if !has_meaningful_article_content(&child) || dom::inner_text(&child).chars().count() < 400 {
            trim_leading_metadata_siblings(&child, metadata);
        }
    }
}

fn trim_leading_metadata_siblings(root: &NodeRef, metadata: &Metadata) {
    for child in root
        .children()
        .filter(|node| node.as_element().is_some())
        .collect::<Vec<_>>()
    {
        if is_leading_metadata_node(&child, metadata) {
            child.detach();
            continue;
        }

        if has_meaningful_article_content(&child) {
            break;
        }

        let text_len = dom::inner_text(&child).chars().count();
        if text_len > 120 {
            break;
        }
    }
}

fn looks_like_article_header(node: &NodeRef, metadata: &Metadata) -> bool {
    if matches!(
        dom::node_name(node).as_str(),
        "body" | "main" | "article" | "section" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
    ) {
        return false;
    }
    let text = dom::inner_text(node);
    let text_len = text.chars().count();
    if text_len > 900 {
        return false;
    }
    if metadata
        .title
        .as_deref()
        .is_some_and(|title| text_similarity(title, &text) > 0.55)
    {
        return true;
    }
    if is_byline_node(node, metadata) || contains_published_time(node, metadata) {
        return true;
    }
    if is_leading_date_node(node) {
        return true;
    }
    !dom::select_nodes(node, "h1, h2, time, address, figure, img, picture").is_empty()
        && dom::select_nodes(node, "p")
            .into_iter()
            .all(|paragraph| dom::inner_text(&paragraph).chars().count() < 120)
}

fn is_leading_metadata_node(node: &NodeRef, metadata: &Metadata) -> bool {
    let tag = dom::node_name(node);
    if matches!(tag.as_str(), "hgroup" | "address" | "time") {
        return true;
    }
    if matches!(tag.as_str(), "figure" | "picture" | "img") && is_hero_media_node(node) {
        return true;
    }
    if matches!(tag.as_str(), "picture" | "img") {
        return false;
    }
    if is_byline_node(node, metadata) || contains_published_time(node, metadata) {
        return true;
    }
    if looks_like_article_header(node, metadata) {
        return true;
    }

    let attrs = metadata_signal_attrs(node);
    if attrs.contains("dek") || attrs.contains("standfirst") || attrs.contains("subtitle") {
        return false;
    }
    let text_len = dom::inner_text(node).chars().count();
    text_len < 220
        && !attrs.is_empty()
        && [
            "byline",
            "author",
            "avatar",
            "dateline",
            "timestamp",
            "meta",
            "metadata",
        ]
        .iter()
        .any(|needle| attrs.contains(needle))
}

fn is_hero_media_node(node: &NodeRef) -> bool {
    let attrs = metadata_signal_attrs(node);
    if ![
        "hero",
        "lead",
        "lede",
        "header",
        "featured",
        "main-image",
        "primary-image",
    ]
    .iter()
    .any(|needle| attrs.contains(needle))
    {
        return false;
    }
    let text_len = dom::inner_text(node).chars().count();
    text_len < 240 && !dom::select_nodes(node, "img, picture, source").is_empty()
}

fn is_byline_node(node: &NodeRef, metadata: &Metadata) -> bool {
    let attrs = metadata_signal_attrs(node);
    let text = dom::inner_text(node);
    let text_len = text.chars().count();
    if text_len == 0 || text_len > 260 {
        return false;
    }
    if metadata
        .byline
        .as_deref()
        .is_some_and(|byline| matches_metadata_value(&text, Some(byline)))
    {
        return true;
    }
    (attrs.contains("byline") || attrs.contains("author") || attrs.contains("dateline")) && text_len < 180
}

fn contains_published_time(node: &NodeRef, metadata: &Metadata) -> bool {
    if !dom::select_nodes(node, "time").is_empty() {
        return true;
    }
    let text = dom::inner_text(node);
    matches_metadata_value(&text, metadata.published_time.as_deref())
}

fn is_leading_date_node(node: &NodeRef) -> bool {
    let text = dom::inner_text(node);
    text.chars().count() < 80 && LEADING_DATE_TEXT.is_match(&text)
}

fn matches_metadata_value(text: &str, metadata_value: Option<&str>) -> bool {
    let Some(metadata_value) = metadata_value else {
        return false;
    };
    let text = text.to_lowercase();
    let metadata_value = metadata_value.to_lowercase();
    !metadata_value.trim().is_empty() && (text.contains(&metadata_value) || metadata_value.contains(text.trim()))
}

fn metadata_signal_attrs(node: &NodeRef) -> String {
    let attrs = dom::attrs(node);
    [
        attrs.get("class").map(String::as_str).unwrap_or_default(),
        attrs.get("id").map(String::as_str).unwrap_or_default(),
        attrs.get("itemprop").map(String::as_str).unwrap_or_default(),
        attrs.get("property").map(String::as_str).unwrap_or_default(),
        attrs.get("data-testid").map(String::as_str).unwrap_or_default(),
        attrs.get("data-component").map(String::as_str).unwrap_or_default(),
    ]
    .join(" ")
    .to_ascii_lowercase()
}

fn text_similarity(a: &str, b: &str) -> f32 {
    let tokens_a: Vec<_> = a
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_lowercase)
        .collect();
    let tokens_b: Vec<_> = b
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_lowercase)
        .collect();
    if tokens_a.is_empty() || tokens_b.is_empty() {
        return 0.0;
    }
    let total_len = tokens_b.join(" ").len().max(1) as f32;
    let unique_b_len = tokens_b
        .iter()
        .filter(|token| !tokens_a.contains(token))
        .cloned()
        .collect::<Vec<_>>()
        .join(" ")
        .len() as f32;
    1.0 - unique_b_len / total_len
}

fn clean_conditionally(root: &NodeRef, options: &ReadabilityOptions, flags: ExtractFlags) {
    for node in dom::select_nodes(root, "table, ul, ol, div, section, header") {
        if dom::node_id(&node) == dom::node_id(root) || dom::has_ancestor_tag(&node, "code", 3) {
            continue;
        }
        if dom::node_name(&node) == "table" && is_data_table(&node) {
            continue;
        }

        let text = dom::inner_text(&node);
        let content_length = text.chars().count();
        if content_length == 0 {
            node.detach();
            continue;
        }

        if AD_OR_LOADING_WORDS.is_match(text.trim()) {
            node.detach();
            continue;
        }

        let weight = class_weight(&node, flags);
        let density = link_density(&node) + options.link_density_modifier as f64;
        let p_count = dom::select_nodes(&node, "p").len();
        let img_count = dom::select_nodes(&node, "img").len();
        let li_count = dom::select_nodes(&node, "li").len().saturating_sub(100);
        let input_count = dom::select_nodes(&node, "input").len();
        let embed_count = dom::select_nodes(&node, "object, embed, iframe").len();
        let comma_count = COMMA.find_iter(&text).count();

        let should_remove = weight < 0
            || (comma_count < 10
                && ((img_count > 1 && p_count.saturating_mul(2) < img_count)
                    || li_count > p_count
                    || input_count > p_count / 3
                    || (content_length < 25 && img_count == 0 && density > 0.0)
                    || (weight < 25 && density > 0.2)
                    || (weight >= 25 && density > 0.5)
                    || (embed_count == 1 && content_length < 75)
                    || embed_count > 1));

        if should_remove {
            node.detach();
        }
    }
}

fn is_data_table(node: &NodeRef) -> bool {
    if dom::attr(node, "role").as_deref() == Some("presentation") {
        return false;
    }

    if dom::attr(node, "datatable").as_deref() == Some("0") {
        return false;
    }

    if dom::attr(node, "summary").is_some()
        || !dom::select_nodes(node, "caption, col, colgroup, tfoot, thead, th").is_empty()
    {
        return true;
    }

    let rows = dom::select_nodes(node, "tr").len();
    let columns = dom::select_nodes(node, "tr")
        .into_iter()
        .map(|row| dom::select_nodes(&row, "td, th").len())
        .max()
        .unwrap_or(0);
    rows >= 2 && columns >= 2 && rows.saturating_mul(columns) >= 10
}

fn remove_empty_blocks(root: &NodeRef) {
    for node in dom::select_nodes(root, "p, div, section, header, h1, h2, h3, h4, h5, h6") {
        if dom::node_id(&node) == dom::node_id(root) {
            continue;
        }
        let has_media = !dom::select_nodes(&node, "img, iframe, video, audio, object, embed").is_empty();
        if !has_media && dom::inner_text(&node).trim().is_empty() {
            node.detach();
        }
    }
}

fn fix_relative_urls(root: &NodeRef, base_url: Option<&Url>) {
    let Some(base_url) = base_url else {
        return;
    };

    for node in dom::select_nodes(root, "a[href], area[href]") {
        if let Some(href) = dom::attr(&node, "href")
            && let Ok(url) = base_url.join(&href)
        {
            dom::set_attr(&node, "href", url.as_str());
        }
    }

    for node in dom::select_nodes(root, "img[src], video[src], audio[src], source[src], iframe[src]") {
        if let Some(src) = dom::attr(&node, "src")
            && let Ok(url) = base_url.join(&src)
        {
            dom::set_attr(&node, "src", url.as_str());
        }
    }
}

fn clean_classes(node: &NodeRef, options: &ReadabilityOptions) {
    if let Some(class) = dom::attr(node, "class") {
        let preserved: Vec<_> = class
            .split_whitespace()
            .filter(|class| {
                DEFAULT_CLASSES_TO_PRESERVE.contains(class)
                    || options.classes_to_preserve.iter().any(|preserve| preserve == class)
            })
            .collect();
        if preserved.is_empty() {
            dom::remove_attr(node, "class");
        } else {
            dom::set_attr(node, "class", &preserved.join(" "));
        }
    }

    for child in node.children() {
        clean_classes(&child, options);
    }
}
