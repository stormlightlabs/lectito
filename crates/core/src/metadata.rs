use std::collections::HashMap;

use scraper::Html;
use url::Url;

use super::config::ReadabilityOptions;
use super::regexes::RegexPattern;
use super::{json_schema, patterns};

// TODO: move to patterns
const TITLE_SEPARATORS: &[&str] = &[" | ", " - ", " – ", " — ", " \\ ", " / ", " > ", " » "];

#[derive(Clone, Debug, Default)]
pub struct Metadata {
    pub title: Option<String>,
    pub byline: Option<String>,
    pub excerpt: Option<String>,
    pub site_name: Option<String>,
    pub published_time: Option<String>,
    pub image: Option<String>,
    pub domain: Option<String>,
    pub favicon: Option<String>,
    pub schema_text: Option<String>,
    pub lang: Option<String>,
    pub dir: Option<String>,
}

pub fn extract_metadata(document: &Html, html: &str, options: &ReadabilityOptions, base_url: Option<&Url>) -> Metadata {
    let mut metadata = if options.disable_json_ld { Metadata::default() } else { json_schema::extract_json_ld(html) };
    let mut values = HashMap::<String, String>::new();
    let meta_selector = patterns::selector("meta");

    for element in document.select(&meta_selector) {
        let content = element
            .value()
            .attr("content")
            .map(str::trim)
            .filter(|content| !content.is_empty());
        let Some(content) = content else {
            continue;
        };

        if let Some(property) = element.value().attr("property") {
            for name in property.split_whitespace() {
                let name = name.to_lowercase();
                if meta_property_key(&name) {
                    values.insert(name, decode_html_entities(content));
                }
            }
        }

        if let Some(name) = element.value().attr("name") {
            let normalized = name.to_lowercase().replace(char::is_whitespace, "").replace('.', ":");
            if meta_name_key(&normalized) {
                values.insert(normalized, decode_html_entities(content));
            }
        }
    }

    metadata.site_name = metadata
        .site_name
        .or_else(|| first_value(&values, &["og:site_name", "application-name"]));
    metadata.title = metadata
        .title
        .or_else(|| {
            first_value(
                &values,
                &[
                    "dc:title",
                    "dcterm:title",
                    "og:title",
                    "weibo:article:title",
                    "weibo:webpage:title",
                    "title",
                    "twitter:title",
                    "parsely-title",
                    "sailthru:title",
                ],
            )
        })
        .and_then(|title| prefer_specific_headline(document, metadata.site_name.as_deref(), &title))
        .or_else(|| article_title(document));
    metadata.byline = metadata
        .byline
        .or_else(|| {
            first_value(
                &values,
                &[
                    "dc:creator",
                    "dcterm:creator",
                    "author",
                    "sailthru:author",
                    "authorlist",
                    "citation_author",
                    "parsely-author",
                    "article:author",
                    "og:article:author",
                ],
            )
            .and_then(|value| normalize_byline(&value))
        })
        .or_else(|| byline_from_document(document));
    metadata.excerpt = metadata.excerpt.or_else(|| {
        first_value(
            &values,
            &[
                "dc:description",
                "dcterm:description",
                "og:description",
                "weibo:article:description",
                "weibo:webpage:description",
                "description",
                "twitter:description",
                "sailthru:description",
            ],
        )
    });
    metadata.published_time = metadata
        .published_time
        .or_else(|| first_value(&values, &["article:published_time", "parsely-pub-date", "publishdate"]));
    metadata.published_time = metadata
        .published_time
        .or_else(|| published_time_from_document(document));
    metadata.image = metadata
        .image
        .or_else(|| first_value(&values, &["og:image", "twitter:image", "sailthru:image:full", "image"]))
        .and_then(|image| absolutize_url(&image, base_url));
    metadata.favicon = metadata
        .favicon
        .or_else(|| first_value(&values, &["og:image:favicon"]))
        .or_else(|| favicon_from_document(document))
        .or_else(|| base_url.and_then(|_| absolutize_url("/favicon.ico", base_url)));
    metadata.favicon = metadata.favicon.and_then(|favicon| absolutize_url(&favicon, base_url));
    metadata.domain = metadata
        .domain
        .or_else(|| canonical_url(document).and_then(|url| domain_from_url(&url)))
        .or_else(|| base_url.and_then(|url| url.host_str().map(strip_www)));

    let html_selector = patterns::selector("html");
    let body_selector = patterns::selector("body");
    let content_dir_selector = patterns::selector(r#"main[dir], [role="main"][dir]"#);
    if let Some(html) = document.select(&html_selector).next() {
        metadata.lang = html.value().attr("lang").map(str::to_string);
        metadata.dir = document
            .select(&body_selector)
            .next()
            .and_then(|body| body.value().attr("dir"))
            .or_else(|| {
                document
                    .select(&content_dir_selector)
                    .next()
                    .and_then(|element| element.value().attr("dir"))
            })
            .or_else(|| html.value().attr("dir"))
            .map(str::to_string);
    }

    metadata
}

pub fn normalize_byline(value: &str) -> Option<String> {
    if is_url(value) {
        return None;
    }

    let mut seen = Vec::<String>::new();
    for part in value.split([',', ';']) {
        let Some(author) = clean_metadata_value(part) else {
            continue;
        };
        if !plausible_byline(&author) {
            continue;
        }
        if !seen.iter().any(|existing| existing.eq_ignore_ascii_case(&author)) {
            seen.push(author);
        }
    }

    (!seen.is_empty()).then(|| seen.join(", "))
}

pub fn clean_metadata_value(value: &str) -> Option<String> {
    let value = decode_html_entities(&patterns::normalize_spaces(value.trim()));
    if value.is_empty() || is_placeholder_value(&value) { None } else { Some(value) }
}

pub fn first_paragraph_excerpt(content: &str) -> Option<String> {
    let document = Html::parse_fragment(content);
    first_excerpt_for_selector(&document, "p").or_else(|| first_excerpt_for_selector(&document, "div"))
}

pub fn decode_html_entities(value: &str) -> String {
    let mut decoded = String::with_capacity(value.len());
    let mut rest = value;

    while let Some(start) = rest.find('&') {
        decoded.push_str(&rest[..start]);
        let after_amp = &rest[start + 1..];
        let Some(end) = after_amp.find(';') else {
            decoded.push_str(&rest[start..]);
            return decoded;
        };

        let entity = &after_amp[..end];
        let replacement = decode_entity(entity);
        if let Some(replacement) = replacement {
            decoded.push_str(&replacement);
        } else {
            decoded.push('&');
            decoded.push_str(entity);
            decoded.push(';');
        }
        rest = &after_amp[end + 1..];
    }

    decoded.push_str(rest);
    decoded
}

fn meta_property_key(name: &str) -> bool {
    let Some((prefix, key)) = name.split_once(':') else {
        return matches!(name, "author" | "description" | "image" | "title");
    };
    matches!(prefix, "article" | "dc" | "dcterm" | "og" | "twitter")
        && matches!(
            key,
            "article:author"
                | "author"
                | "creator"
                | "description"
                | "image"
                | "image:favicon"
                | "published_time"
                | "title"
                | "site_name"
        )
}

fn meta_name_key(name: &str) -> bool {
    let name = name
        .strip_prefix("dc:")
        .or_else(|| name.strip_prefix("dcterm:"))
        .or_else(|| name.strip_prefix("og:"))
        .or_else(|| name.strip_prefix("twitter:"))
        .or_else(|| name.strip_prefix("parsely-"))
        .or_else(|| name.strip_prefix("sailthru:"))
        .or_else(|| name.strip_prefix("weibo:article:"))
        .or_else(|| name.strip_prefix("weibo:webpage:"))
        .unwrap_or(name);
    matches!(
        name,
        "application-name"
            | "author"
            | "authorlist"
            | "citation_author"
            | "creator"
            | "image"
            | "image:full"
            | "pub-date"
            | "publishdate"
            | "description"
            | "title"
            | "site_name"
    )
}

fn article_title(document: &Html) -> Option<String> {
    let title_selector = patterns::selector("title");
    let title = document
        .select(&title_selector)
        .next()
        .map(|title| title.text().collect::<String>())
        .unwrap_or_default();
    let original = patterns::normalize_spaces(title.trim());
    if original.is_empty() {
        return Some(String::new());
    }

    let mut had_hierarchical_separator = false;
    let mut title = original.clone();

    if let Some((separator, index)) = last_separator(&original) {
        had_hierarchical_separator = matches!(separator, " \\ " | " / " | " > " | " » ");
        title = patterns::normalize_spaces(original[..index].trim());

        if word_count(&title) < 3 {
            title = patterns::normalize_spaces(original[index + separator.len()..].trim());
        }
    } else if original.contains(": ") && !heading_matches(document, &original) {
        title = patterns::normalize_spaces(original[original.rfind(':').unwrap_or(0) + 1..].trim());
        if word_count(&title) < 3 {
            title = patterns::normalize_spaces(original[original.find(':').unwrap_or(0) + 1..].trim());
        } else if word_count(&original[..original.find(':').unwrap_or(0)]) > 5 {
            title = original.clone();
        }
    } else if original.chars().count() > 150 || original.chars().count() < 15 {
        let h1_selector = patterns::selector("h1");
        let h1s: Vec<_> = document.select(&h1_selector).collect();
        if h1s.len() == 1 {
            title = patterns::normalize_spaces(h1s[0].text().collect::<String>().trim());
        }
    }

    let title_word_count = word_count(&title);
    let original_without_separators = TITLE_SEPARATORS
        .iter()
        .fold(original.clone(), |title, separator| title.replace(separator, " "));
    if title_word_count <= 4
        && (!had_hierarchical_separator || title_word_count != word_count(&original_without_separators) - 1)
    {
        title = original;
    }

    Some(title)
}

fn prefer_specific_headline(document: &Html, site_name: Option<&str>, title: &str) -> Option<String> {
    let title = clean_title_chrome(&patterns::normalize_spaces(title.trim()));
    if title.is_empty() {
        return None;
    }

    let headline = specific_heading(document);
    if let (Some(site_name), Some(headline)) = (site_name, headline.as_deref())
        && title.eq_ignore_ascii_case(site_name)
    {
        return Some(headline.to_string());
    }

    if word_count(&title) <= 2
        && let Some(headline) = headline.filter(|headline| word_count(headline) >= 3)
    {
        return Some(headline);
    }

    Some(title)
}

fn clean_title_chrome(title: &str) -> String {
    ["Stay organized with collections Save and categorize content based on your preferences."]
        .iter()
        .fold(title.to_string(), |title, suffix| {
            title
                .strip_suffix(suffix)
                .map(|title| patterns::normalize_spaces(title.trim().trim_matches(['-', '|', '•']).trim()))
                .unwrap_or(title)
        })
}

fn specific_heading(document: &Html) -> Option<String> {
    let selector = patterns::selector(r#"article h1, main h1, [role="main"] h1, h1"#);
    document
        .select(&selector)
        .map(|heading| patterns::normalize_spaces(heading.text().collect::<String>().trim()))
        .find(|heading| !heading.is_empty())
}

fn last_separator(title: &str) -> Option<(&'static str, usize)> {
    TITLE_SEPARATORS
        .iter()
        .filter_map(|separator| title.rfind(separator).map(|index| (*separator, index)))
        .max_by_key(|(_, index)| *index)
}

fn word_count(value: &str) -> usize {
    value.split_whitespace().count()
}

fn heading_matches(document: &Html, title: &str) -> bool {
    let selector = patterns::selector("h1, h2");
    document
        .select(&selector)
        .any(|heading| patterns::normalize_spaces(heading.text().collect::<String>().trim()) == title)
}

fn byline_from_document(document: &Html) -> Option<String> {
    if let Some(byline) = byline_from_latexml(document) {
        return Some(byline);
    }

    for selector in [
        r#"[itemprop*="author"] [itemprop*="name"], [rel="author"] [itemprop*="name"], a[rel="author"], [class*="author"] a[href*="/author/"], [class*="byline"] a[href*="/author/"]"#,
        r#"[rel="author"], [itemprop*="author"]"#,
        r#".byline, .article-author, .p-author, [class*="byline"], [id*="byline"], [id*="author"], .author, [class*="author"]"#,
    ] {
        if let Some(byline) = byline_from_selector(document, selector) {
            return Some(byline);
        }
    }
    None
}

fn byline_from_selector(document: &Html, selector: &str) -> Option<String> {
    let selector = patterns::selector(selector);
    for element in document.select(&selector) {
        if byline_element_is_chrome(&element) {
            continue;
        }
        let text = if element
            .value()
            .attr("itemprop")
            .is_some_and(|itemprop| itemprop.contains("author"))
        {
            let name_selector = patterns::selector(r#"[itemprop*="name"]"#);
            element
                .select(&name_selector)
                .next()
                .map(|name| name.text().collect::<String>())
                .unwrap_or_else(|| element.text().collect::<String>())
        } else {
            element.text().collect::<String>()
        };
        let byline = clean_byline(&text);
        if plausible_byline(&byline) {
            return Some(byline);
        }
    }
    None
}

fn byline_from_latexml(document: &Html) -> Option<String> {
    let selector = patterns::selector(".ltx_authors .ltx_personname");
    for element in document.select(&selector) {
        let text = element.text().collect::<Vec<_>>().join("\n");
        let mut names = Vec::new();
        for chunk in text.split('&') {
            let mut lines = chunk
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.contains('@'));
            let Some(name) = lines.next() else {
                continue;
            };
            let Some(name) = clean_metadata_value(name) else {
                continue;
            };
            if plausible_byline(&name)
                && !names
                    .iter()
                    .any(|existing: &String| existing.eq_ignore_ascii_case(&name))
            {
                names.push(name);
            }
        }
        if !names.is_empty() {
            return Some(names.join(", "));
        }
    }
    None
}

fn byline_element_is_chrome(element: &scraper::ElementRef<'_>) -> bool {
    element
        .ancestors()
        .filter_map(scraper::ElementRef::wrap)
        .any(|ancestor| {
            let attrs = [
                ancestor.value().attr("class").unwrap_or_default(),
                ancestor.value().attr("id").unwrap_or_default(),
                ancestor.value().attr("data-component").unwrap_or_default(),
                ancestor.value().attr("data-testid").unwrap_or_default(),
            ]
            .join(" ")
            .to_ascii_lowercase();
            [
                "backlink",
                "comment",
                "follow-author",
                "mention",
                "profile",
                "reply",
                "webmention",
            ]
            .iter()
            .any(|needle| attrs.contains(needle))
        })
}

fn clean_byline(value: &str) -> String {
    let value = patterns::normalize_spaces(value.trim());
    let value = RegexPattern::BylinePrefix.to_regex().replace(&value, "");
    let value = RegexPattern::BylineTrailingDate.to_regex().replace(&value, "");
    patterns::normalize_spaces(value.trim().trim_matches(['-', '|', '•']).trim())
}

fn plausible_byline(value: &str) -> bool {
    let lower = value.to_lowercase();
    !value.is_empty()
        && value.chars().count() < 140
        && !matches!(
            lower.as_str(),
            "author" | "authors" | "by" | "byline" | "follow author" | "about the author"
        )
        && !lower.contains("work done exclusively")
        && !lower.contains("authors ordered")
}

fn is_placeholder_value(value: &str) -> bool {
    let lower = value.trim().to_lowercase();
    lower.contains("{{")
        || lower.contains("}}")
        || lower.contains("#{")
        || lower == "null"
        || lower == "undefined"
        || lower == "n/a"
}

fn favicon_from_document(document: &Html) -> Option<String> {
    let selector = patterns::selector(r#"link[rel~="icon"], link[rel="shortcut icon"], link[rel="apple-touch-icon"]"#);
    document
        .select(&selector)
        .find_map(|link| link.value().attr("href").and_then(clean_metadata_value))
}

fn canonical_url(document: &Html) -> Option<String> {
    let selector = patterns::selector(r#"link[rel="canonical"]"#);
    document
        .select(&selector)
        .find_map(|link| link.value().attr("href").and_then(clean_metadata_value))
}

fn published_time_from_document(document: &Html) -> Option<String> {
    let selector = patterns::selector(
        r#"time[datetime], [itemprop*="datePublished"][datetime], [itemprop*="datePublished"][content], [property="article:published_time"][content]"#,
    );
    document.select(&selector).find_map(|element| {
        element
            .value()
            .attr("datetime")
            .or_else(|| element.value().attr("content"))
            .and_then(clean_metadata_value)
    })
}

fn absolutize_url(value: &str, base_url: Option<&Url>) -> Option<String> {
    let value = clean_metadata_value(value)?;
    if let Ok(url) = Url::parse(&value) {
        return Some(url.to_string());
    }
    base_url.and_then(|base_url| base_url.join(&value).ok().map(|url| url.to_string()))
}

fn domain_from_url(value: &str) -> Option<String> {
    Url::parse(value).ok()?.host_str().map(strip_www)
}

fn strip_www(value: &str) -> String {
    value.strip_prefix("www.").unwrap_or(value).to_string()
}

fn first_excerpt_for_selector(document: &Html, selector_pattern: &str) -> Option<String> {
    let selector = patterns::selector(selector_pattern);
    document
        .select(&selector)
        .filter(|element| element.value().attr("id") != Some("readability-page-1"))
        .map(|element| decode_html_entities(&patterns::normalize_spaces(element.text().collect::<String>().trim())))
        .find(|excerpt| {
            let len = excerpt.chars().count();
            (15..=1000).contains(&len)
        })
        .filter(|excerpt| !excerpt.is_empty())
}

fn first_value(values: &HashMap<String, String>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| values.get(*key).cloned())
        .filter(|value| !value.trim().is_empty())
}

fn decode_entity(entity: &str) -> Option<String> {
    match entity {
        "amp" => Some("&".to_string()),
        "apos" => Some("'".to_string()),
        "gt" => Some(">".to_string()),
        "lt" => Some("<".to_string()),
        "nbsp" => Some(" ".to_string()),
        "quot" => Some("\"".to_string()),
        _ if entity.starts_with("#x") || entity.starts_with("#X") => decode_numeric_entity(&entity[2..], 16),
        _ if entity.starts_with('#') => decode_numeric_entity(&entity[1..], 10),
        _ => None,
    }
}

fn decode_numeric_entity(value: &str, radix: u32) -> Option<String> {
    let codepoint = u32::from_str_radix(value, radix).ok()?;
    let character = char::from_u32(codepoint)
        .filter(|character| *character != '\0')
        .unwrap_or('\u{fffd}');
    Some(character.to_string())
}

fn is_url(value: &str) -> bool {
    Url::parse(value).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_h1_when_meta_title_is_site_name_even_for_short_headlines() {
        let html = r#"
            <html><head>
                <meta property="og:site_name" content="Simon Willison’s Weblog">
                <meta property="og:title" content="Simon Willison’s Weblog">
                <title>Datasette Agent</title>
            </head><body><article><h1>Datasette Agent</h1></article></body></html>
        "#;
        let document = Html::parse_document(html);
        let metadata = extract_metadata(&document, html, &ReadabilityOptions::default(), None);

        assert_eq!(metadata.title.as_deref(), Some("Datasette Agent"));
        assert_eq!(metadata.site_name.as_deref(), Some("Simon Willison’s Weblog"));
    }

    #[test]
    fn removes_web_dev_collection_ui_from_titles() {
        let html = r#"
            <html><head>
                <meta property="og:title" content="Responsive images Stay organized with collections Save and categorize content based on your preferences.">
            </head><body><main><h1>Responsive images</h1></main></body></html>
        "#;
        let document = Html::parse_document(html);
        let metadata = extract_metadata(&document, html, &ReadabilityOptions::default(), None);

        assert_eq!(metadata.title.as_deref(), Some("Responsive images"));
    }
}
