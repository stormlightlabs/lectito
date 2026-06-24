mod hn;

use hn::HackerNewsExtractor;
use kuchiki::NodeRef;
use regex::Regex;
use serde::Deserialize;
use url::Url;

use super::config::{ExtractFlags, ReadabilityOptions};
use super::diagnostics::{SiteRuleDiagnostic, SiteRuleMatch, SiteRuleSource};
use super::error::{Error, Result};
use super::extract::{ExtractAttempt, element_count};
use super::metadata::Metadata;
use super::{dom, normalize, serialize};

const BUNDLED_PROFILES: &[(&str, &str)] = &[
    ("wikipedia.org.toml", include_str!("./rules/conf/wikipedia.org.toml")),
    ("mozilla.org.toml", include_str!("./rules/conf/mozilla.org.toml")),
    ("github.com.toml", include_str!("./rules/conf/github.com.toml")),
    ("sre.google.toml", include_str!("./rules/conf/sre.google.toml")),
    (
        "plato.stanford.edu.toml",
        include_str!("./rules/conf/plato.stanford.edu.toml"),
    ),
    ("readthedocs.io.toml", include_str!("./rules/conf/readthedocs.io.toml")),
];

static HACKER_NEWS_EXTRACTOR: HackerNewsExtractor = HackerNewsExtractor;

trait SiteExtractor {
    fn name(&self) -> &'static str;
    fn matches(&self, url: &Url) -> bool;

    fn extract(
        &self, doc: &NodeRef, url: &Url, opts: &ReadabilityOptions, metadata: &Metadata,
    ) -> Result<Option<ExtractAttempt>>;
}

pub struct RuleExtraction {
    pub attempt: ExtractAttempt,
    pub flags: ExtractFlags,
    pub diagnostic: SiteRuleDiagnostic,
}

#[derive(Clone, Debug, Default)]
struct SiteProfile {
    name: String,
    hosts: Vec<String>,
    subdomains: bool,
    path_prefixes: Vec<String>,
    exclude_path_prefixes: Vec<String>,
    content_roots: Vec<String>,
    remove: Vec<String>,
    remove_id_or_class: Vec<String>,
    metadata: MetadataProfile,
    cleanup: CleanupProfile,
    fallback: FallbackProfile,
    bundled: bool,
    specificity: usize,
}

#[derive(Clone, Debug, Default)]
struct MetadataProfile {
    title: Vec<String>,
    author: Vec<String>,
    date: Vec<String>,
    image: Vec<String>,
    site_name: Option<String>,
    title_suffixes: Vec<String>,
}

#[derive(Clone, Debug)]
struct CleanupProfile {
    enabled: bool,
    prune: bool,
}

impl Default for CleanupProfile {
    fn default() -> Self {
        Self { enabled: true, prune: true }
    }
}

#[derive(Clone, Debug)]
struct FallbackProfile {
    generic_on_empty: bool,
}

impl Default for FallbackProfile {
    fn default() -> Self {
        Self { generic_on_empty: true }
    }
}

#[derive(Clone, Debug)]
struct SelectorQuery {
    selector: String,
    attr: Option<String>,
}

#[derive(Clone, Debug)]
struct ProfileMatch {
    profile: SiteProfile,
    host: String,
    path_prefix: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct TomlSiteProfile {
    name: Option<String>,
    hosts: Vec<String>,
    #[serde(default)]
    subdomains: bool,
    #[serde(default)]
    path_prefixes: Vec<String>,
    #[serde(default)]
    exclude_path_prefixes: Vec<String>,
    #[serde(default, alias = "content_roots")]
    content_roots: Vec<String>,
    #[serde(default)]
    remove: Vec<String>,
    #[serde(default)]
    remove_id_or_class: Vec<String>,
    #[serde(default)]
    metadata: TomlMetadataProfile,
    #[serde(default)]
    cleanup: TomlCleanupProfile,
    #[serde(default)]
    fallback: TomlFallbackProfile,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct TomlMetadataProfile {
    #[serde(default)]
    title: Vec<String>,
    #[serde(default, alias = "byline")]
    author: Vec<String>,
    #[serde(default, alias = "published_time")]
    date: Vec<String>,
    #[serde(default)]
    image: Vec<String>,
    site_name: Option<String>,
    #[serde(default)]
    title_suffixes: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct TomlCleanupProfile {
    enabled: Option<bool>,
    prune: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct TomlFallbackProfile {
    generic_on_empty: Option<bool>,
}

pub fn extract_with_site_rule(
    doc: &NodeRef, url: Option<&Url>, opts: &ReadabilityOptions, metadata: &Metadata,
) -> Result<Option<RuleExtraction>> {
    let Some(url) = url else {
        return Ok(None);
    };
    if opts.content_selector.is_some() {
        return Ok(None);
    }

    if let Some(profile_match) = matching_profile(url, opts)?
        && let Some(extraction) = extract_with_profile(doc, url, opts, metadata, profile_match)?
    {
        return Ok(Some(extraction));
    }

    for extractor in [&HACKER_NEWS_EXTRACTOR] {
        if extractor.matches(url)
            && let Some(attempt) = extractor.extract(doc, url, opts, metadata)?
        {
            let flags = ExtractFlags { strip_unlikely: false, weight_classes: false, clean_conditionally: false };
            let diagnostic = SiteRuleDiagnostic {
                name: extractor.name().to_string(),
                source: SiteRuleSource::CodeExtractor,
                matched_by: SiteRuleMatch {
                    host: url.host_str().unwrap_or_default().to_string(),
                    path_prefix: None,
                    bundled: true,
                },
                roots: Vec::new(),
                removals: 0,
                text_len: attempt.text_len,
                accepted: false,
                fallback_reason: None,
            };
            return Ok(Some(RuleExtraction { attempt, flags, diagnostic }));
        }
    }

    Ok(None)
}

fn extract_with_profile(
    doc: &NodeRef, url: &Url, opts: &ReadabilityOptions, metadata: &Metadata, profile_match: ProfileMatch,
) -> Result<Option<RuleExtraction>> {
    let profile = profile_match.profile;
    let removals = apply_removals(doc, &profile);
    let roots = select_first_non_empty(doc, &profile.content_roots);
    if roots.is_empty() {
        if profile.fallback.generic_on_empty {
            return Ok(None);
        }
        return Ok(Some(RuleExtraction {
            attempt: empty_attempt(metadata.clone()),
            flags: profile_flags(&profile),
            diagnostic: SiteRuleDiagnostic {
                name: profile.name,
                source: SiteRuleSource::DeclarativeProfile,
                matched_by: SiteRuleMatch {
                    host: profile_match.host,
                    path_prefix: profile_match.path_prefix,
                    bundled: profile.bundled,
                },
                roots: Vec::new(),
                removals,
                text_len: 0,
                accepted: false,
                fallback_reason: Some("profile matched but no content roots matched".to_string()),
            },
        }));
    }

    let mut metadata = metadata.clone();
    apply_metadata_hints(doc, &profile, &mut metadata);

    let root_selectors = roots.iter().map(node_selector).collect::<Vec<_>>();
    let flags = profile_flags(&profile);
    let attempt = serialize_profile_roots(roots, opts, flags, url.into(), metadata, profile.cleanup.enabled)?;
    let diagnostic = SiteRuleDiagnostic {
        name: profile.name,
        source: SiteRuleSource::DeclarativeProfile,
        matched_by: SiteRuleMatch {
            host: profile_match.host,
            path_prefix: profile_match.path_prefix,
            bundled: profile.bundled,
        },
        roots: root_selectors,
        removals,
        text_len: attempt.text_len,
        accepted: false,
        fallback_reason: None,
    };

    Ok(Some(RuleExtraction { attempt, flags, diagnostic }))
}

fn apply_metadata_hints(doc: &NodeRef, profile: &SiteProfile, metadata: &mut Metadata) {
    if let Some(title) = extract_string(doc, &profile.metadata.title) {
        metadata.title = Some(strip_title_suffixes(title, &profile.metadata.title_suffixes));
    }
    if let Some(author) = extract_string(doc, &profile.metadata.author) {
        metadata.byline = Some(author);
    }
    if let Some(date) = extract_string(doc, &profile.metadata.date) {
        metadata.published_time = Some(date);
    }
    if let Some(image) = extract_string(doc, &profile.metadata.image) {
        metadata.image = Some(image);
    }
    if let Some(site_name) = &profile.metadata.site_name {
        metadata.site_name = Some(site_name.clone());
        if metadata.byline.is_none() && site_name == "Wikipedia" {
            metadata.byline = Some(site_name.clone());
        }
    }
}

fn matching_profile(url: &Url, opts: &ReadabilityOptions) -> Result<Option<ProfileMatch>> {
    let Some(host) = url.host_str().map(|host| host.trim_start_matches("www.").to_string()) else {
        return Ok(None);
    };
    let path = url.path();
    let mut matches = Vec::new();

    for (index, source) in opts.site_profiles.iter().enumerate() {
        let mut profile = parse_toml_profile(&format!("user-profile-{index}"), source, false)?;
        if let Some(path_prefix) = matching_profile_path(&profile, &host, path) {
            profile.specificity += 10_000usize.saturating_sub(index);
            matches.push(ProfileMatch { profile, host: host.clone(), path_prefix });
        }
    }

    for (index, (name, source)) in BUNDLED_PROFILES.iter().enumerate() {
        let mut profile = parse_toml_profile(name, source, true)?;
        if let Some(path_prefix) = matching_profile_path(&profile, &host, path) {
            profile.specificity += 1_000usize.saturating_sub(index);
            matches.push(ProfileMatch { profile, host: host.clone(), path_prefix });
        }
    }

    matches.sort_by_key(|b| std::cmp::Reverse(b.profile.specificity));
    Ok(matches.into_iter().next())
}

fn matching_profile_path(profile: &SiteProfile, host: &str, path: &str) -> Option<Option<String>> {
    if !profile
        .hosts
        .iter()
        .any(|pattern| host_matches(pattern, profile.subdomains, host))
    {
        return None;
    }
    if profile
        .exclude_path_prefixes
        .iter()
        .any(|prefix| path.starts_with(prefix.as_str()))
    {
        return None;
    }
    if profile.path_prefixes.is_empty() {
        return Some(None);
    }
    profile
        .path_prefixes
        .iter()
        .find(|prefix| path.starts_with(prefix.as_str()))
        .cloned()
        .map(Some)
}

fn host_matches(pattern: &str, subdomains: bool, host: &str) -> bool {
    let pattern = pattern.trim().trim_start_matches("www.");
    host == pattern || (subdomains && host.ends_with(&format!(".{pattern}")))
}

fn parse_toml_profile(name: &str, source: &str, bundled: bool) -> Result<SiteProfile> {
    let profile: TomlSiteProfile = toml::from_str(source)
        .map_err(|error| Error::InvalidSiteProfile { name: name.to_string(), message: error.to_string() })?;
    if profile.hosts.is_empty() {
        return Err(Error::InvalidSiteProfile {
            name: name.to_string(),
            message: "profile must define at least one host".to_string(),
        });
    }
    if profile.content_roots.is_empty() {
        return Err(Error::InvalidSiteProfile {
            name: name.to_string(),
            message: "profile must define at least one content root".to_string(),
        });
    }

    Ok(SiteProfile {
        name: profile.name.unwrap_or_else(|| name.to_string()),
        specificity: profile.hosts.iter().map(|host| host.len()).max().unwrap_or_default()
            + usize::from(!profile.subdomains) * 100
            + profile
                .path_prefixes
                .iter()
                .map(|path| path.len())
                .max()
                .unwrap_or_default(),
        hosts: profile.hosts,
        subdomains: profile.subdomains,
        path_prefixes: profile.path_prefixes,
        exclude_path_prefixes: profile.exclude_path_prefixes,
        content_roots: profile.content_roots,
        remove: profile.remove,
        remove_id_or_class: profile.remove_id_or_class,
        metadata: MetadataProfile {
            title: profile.metadata.title,
            author: profile.metadata.author,
            date: profile.metadata.date,
            image: profile.metadata.image,
            site_name: profile.metadata.site_name,
            title_suffixes: profile.metadata.title_suffixes,
        },
        cleanup: CleanupProfile {
            enabled: profile.cleanup.enabled.unwrap_or(true),
            prune: profile.cleanup.prune.unwrap_or(true),
        },
        fallback: FallbackProfile { generic_on_empty: profile.fallback.generic_on_empty.unwrap_or(true) },
        bundled,
    })
}

fn apply_removals(doc: &NodeRef, profile: &SiteProfile) -> usize {
    let mut removals = 0;
    for pattern in &profile.remove_id_or_class {
        for node in dom::select_nodes(doc, "*") {
            let id_matches = dom::attr(&node, "id").as_deref() == Some(pattern.as_str());
            let class_matches =
                dom::attr(&node, "class").is_some_and(|class| class.split_whitespace().any(|token| token == pattern));
            if id_matches || class_matches {
                node.detach();
                removals += 1;
            }
        }
    }

    for selector in &profile.remove {
        if let Some(query) = selector_to_query(selector) {
            for node in dom::select_nodes(doc, &query.selector) {
                if let Some(attr) = query.attr.as_deref() {
                    dom::remove_attr(&node, attr);
                } else {
                    node.detach();
                }
                removals += 1;
            }
        }
    }
    removals
}

fn select_first_non_empty(doc: &NodeRef, selectors: &[String]) -> Vec<NodeRef> {
    for selector in selectors {
        let Some(query) = selector_to_query(selector) else {
            continue;
        };
        if query.attr.is_some() {
            continue;
        }
        let nodes = dom::select_nodes(doc, &query.selector)
            .into_iter()
            .filter(|node| !dom::inner_text(node).is_empty())
            .collect::<Vec<_>>();
        if !nodes.is_empty() {
            return nodes;
        }
    }
    Vec::new()
}

fn extract_string(doc: &NodeRef, selectors: &[String]) -> Option<String> {
    for selector in selectors {
        let Some(query) = selector_to_query(selector) else {
            continue;
        };
        for node in dom::select_nodes(doc, &query.selector) {
            let value = query
                .attr
                .as_deref()
                .and_then(|attr| dom::attr(&node, attr))
                .unwrap_or_else(|| dom::inner_text(&node));
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn selector_to_query(selector: &str) -> Option<SelectorQuery> {
    let selector = selector.trim();
    if selector.starts_with('/') {
        xpath_to_query(selector)
    } else {
        Some(SelectorQuery { selector: selector.to_string(), attr: None })
    }
}

fn xpath_to_query(xpath: &str) -> Option<SelectorQuery> {
    let mut xpath = xpath.trim();
    let mut attr = None;

    if let Some(stripped) = xpath.strip_suffix("/text()") {
        xpath = stripped;
    }
    if let Some((prefix, attr_name)) = xpath.rsplit_once("/@") {
        xpath = prefix;
        attr = Some(attr_name.to_string());
    }

    let mut parts = Vec::new();
    for raw in xpath.trim_start_matches('/').split("//") {
        let raw = raw.trim_matches('/');
        if raw.is_empty() {
            continue;
        }
        parts.push(xpath_segment_to_css(raw)?);
    }

    if parts.is_empty() {
        return None;
    }

    Some(SelectorQuery { selector: parts.join(" "), attr })
}

fn xpath_segment_to_css(segment: &str) -> Option<String> {
    let plain = Regex::new(r"^([A-Za-z][\w-]*|\*)$").ok()?;
    if plain.is_match(segment) {
        return Some(css_tag(segment));
    }

    let eq = Regex::new(r#"^([A-Za-z][\w-]*|\*)\[@([\w:-]+)='([^']+)'\]$"#).ok()?;
    if let Some(caps) = eq.captures(segment) {
        let tag = caps.get(1)?.as_str();
        let attr = caps.get(2)?.as_str();
        let value = caps.get(3)?.as_str();
        return match attr {
            "id" => Some(if tag == "*" { format!("#{value}") } else { format!("{tag}#{value}") }),
            "class" => {
                let classes = value
                    .split_whitespace()
                    .filter(|class| !class.is_empty())
                    .map(|class| format!(".{class}"))
                    .collect::<String>();
                Some(format!("{}{classes}", css_tag(tag)))
            }
            _ => Some(format!(r#"{}[{attr}="{value}"]"#, css_tag(tag))),
        };
    }

    let contains = Regex::new(r#"^([A-Za-z][\w-]*|\*)\[contains\(@([\w:-]+), '([^']+)'\)\]$"#).ok()?;
    if let Some(caps) = contains.captures(segment) {
        let tag = caps.get(1)?.as_str();
        let attr = caps.get(2)?.as_str();
        let value = caps.get(3)?.as_str();
        return Some(format!(r#"{}[{attr}*="{value}"]"#, css_tag(tag)));
    }

    None
}

fn css_tag(tag: &str) -> String {
    if tag == "*" { String::new() } else { tag.to_string() }
}

fn strip_title_suffixes(title: String, suffixes: &[String]) -> String {
    let mut title = title.trim().to_string();
    for suffix in suffixes {
        title = title.trim_end_matches(suffix).trim().to_string();
    }
    title
}

// TODO: this could be an into/from
fn profile_flags(profile: &SiteProfile) -> ExtractFlags {
    ExtractFlags { strip_unlikely: false, weight_classes: false, clean_conditionally: profile.cleanup.prune }
}

fn serialize_profile_roots(
    roots: Vec<NodeRef>, opts: &ReadabilityOptions, flags: ExtractFlags, base_url: Option<&Url>, metadata: Metadata,
    cleanup: bool,
) -> Result<ExtractAttempt> {
    if cleanup {
        super::cleanup::cleanup_article(&roots, opts, flags, base_url, &metadata);
    } else {
        for root in &roots {
            super::cleanup::fix_relative_urls(root, base_url);
        }
    }
    normalize::normalize_article(&roots, metadata.title.as_deref());

    let mut content = String::from(r#"<div id="readability-page-1" class="page">"#);
    for node in &roots {
        if dom::node_name(node) == "body" {
            content.push_str(&serialize::serialize_children(node)?);
        } else {
            content.push_str(&serialize::serialize_node(node)?);
        }
    }
    content.push_str("</div>");
    let text_content = serialize::text_content(&roots);
    let text_len = text_content.encode_utf16().count();

    if text_len == 0 && roots.iter().map(element_count).sum::<usize>() == 0 {
        return Ok(empty_attempt(metadata));
    }

    Ok(ExtractAttempt { metadata, content, text_content, text_len })
}

// TODO: this could be an into/from
fn empty_attempt(metadata: Metadata) -> ExtractAttempt {
    ExtractAttempt { metadata, content: String::new(), text_content: String::new(), text_len: 0 }
}

fn node_selector(node: &NodeRef) -> String {
    let tag = dom::node_name(node);
    if tag.is_empty() {
        return "<node>".to_string();
    }
    if let Some(id) = dom::attr(node, "id")
        && !id.trim().is_empty()
    {
        return format!("{tag}#{id}");
    }
    if let Some(class) = dom::attr(node, "class") {
        let mut classes = class.split_whitespace().take(3).collect::<Vec<_>>();
        if !classes.is_empty() {
            classes.insert(0, tag.as_str());
            return classes.join(".");
        }
    }
    tag
}

#[cfg(test)]
mod tests {
    use super::*;
    use kuchiki::traits::TendrilSink;

    #[test]
    fn converts_focused_xpath_subset_to_css() {
        assert_eq!(
            xpath_to_query("//div[@id='bodyContent']").unwrap().selector,
            "div#bodyContent"
        );
        assert_eq!(
            xpath_to_query("//div[@id='article-content']").unwrap().selector,
            "div#article-content"
        );
        assert_eq!(
            xpath_to_query("//article[contains(@class, 'markdown-body')]")
                .unwrap()
                .selector,
            r#"article[class*="markdown-body"]"#
        );
        let query = xpath_to_query("//p[@class='entry-posted']//abbr[@class='published']/@title").unwrap();
        assert_eq!(query.selector, "p.entry-posted abbr.published");
        assert_eq!(query.attr.as_deref(), Some("title"));
    }

    #[test]
    fn dotted_rules_match_subdomains() {
        assert!(host_matches("wikipedia.org", true, "en.wikipedia.org"));
        assert!(host_matches("plato.stanford.edu", false, "plato.stanford.edu"));

        let options = ReadabilityOptions::default();
        let rule = matching_profile(
            &Url::parse("https://plato.stanford.edu/entries/supervenience/").unwrap(),
            &options,
        )
        .unwrap()
        .unwrap()
        .profile;
        assert_eq!(rule.name, "stanford-encyclopedia");
        assert_eq!(
            rule.content_roots.first().map(String::as_str),
            Some("//div[@id='article-content']")
        );
    }

    #[test]
    fn bundled_profiles_are_valid_toml_profiles() {
        for (name, source) in BUNDLED_PROFILES {
            let profile = parse_toml_profile(name, source, true).unwrap();
            assert!(!profile.hosts.is_empty(), "{name} hosts");
            assert!(!profile.content_roots.is_empty(), "{name} content roots");
            assert!(profile.bundled, "{name} bundled");
        }
    }

    #[test]
    fn user_profile_overrides_bundled_profile() {
        let options = ReadabilityOptions {
            site_profiles: vec![
                r##"
                name = "custom wikipedia"
                hosts = ["wikipedia.org"]
                subdomains = true
                content_roots = ["#custom"]
                "##
                .to_string(),
            ],
            ..Default::default()
        };
        let rule = matching_profile(&Url::parse("https://en.wikipedia.org/wiki/Rust").unwrap(), &options)
            .unwrap()
            .unwrap()
            .profile;
        assert_eq!(rule.name, "custom wikipedia");
        assert_eq!(rule.content_roots, vec!["#custom"]);
    }

    #[test]
    fn path_specific_profile_beats_host_profile() {
        let options = ReadabilityOptions {
            site_profiles: vec![
                r#"
                name = "host"
                hosts = ["example.com"]
                content_roots = ["article"]
                "#
                .to_string(),
                r#"
                name = "docs"
                hosts = ["example.com"]
                path_prefixes = ["/docs"]
                content_roots = ["main"]
                "#
                .to_string(),
            ],
            ..Default::default()
        };
        let rule = matching_profile(&Url::parse("https://example.com/docs/page").unwrap(), &options)
            .unwrap()
            .unwrap()
            .profile;
        assert_eq!(rule.name, "docs");
    }

    #[test]
    fn path_exclusions_keep_hacker_news_items_on_code_extractor() {
        let options = ReadabilityOptions {
            site_profiles: vec![
                r##"
                name = "listing"
                hosts = ["news.ycombinator.com"]
                path_prefixes = ["/"]
                exclude_path_prefixes = ["/item"]
                content_roots = ["#bigbox"]
                "##
                .to_string(),
            ],
            ..Default::default()
        };
        let listing = matching_profile(&Url::parse("https://news.ycombinator.com/news").unwrap(), &options)
            .unwrap()
            .unwrap()
            .profile;
        assert_eq!(listing.name, "listing");

        let item = matching_profile(
            &Url::parse("https://news.ycombinator.com/item?id=38646892").unwrap(),
            &options,
        )
        .unwrap();
        assert!(item.is_none());
    }

    #[test]
    fn selects_hyphenated_id_rule_body() {
        let document = kuchiki::parse_html().one("<html><body><div id='article-content'>SEP body</div></body></html>");
        let nodes = select_first_non_empty(&document, &["//div[@id='article-content']".to_string()]);
        assert_eq!(nodes.len(), 1);
        assert_eq!(dom::inner_text(&nodes[0]), "SEP body");
    }

    #[test]
    fn extracts_stanford_article_content_rule() {
        let document = kuchiki::parse_html().one(
            "<html><body><div id='container'>chrome</div><div id='article-content'><h1>SEP</h1><p>Real body text.</p></div></body></html>",
        );
        let url = Url::parse("https://plato.stanford.edu/entries/supervenience/").unwrap();
        let extraction = extract_with_site_rule(
            &document,
            Some(&url),
            &ReadabilityOptions::default(),
            &Metadata::default(),
        )
        .unwrap()
        .unwrap();
        assert!(extraction.attempt.content.contains("Real body text."));
        assert!(!extraction.attempt.content.contains("chrome"));
        assert_eq!(extraction.diagnostic.source, SiteRuleSource::DeclarativeProfile);
    }

    #[test]
    fn extracts_google_sre_content_rule() {
        let document = kuchiki::parse_html().one(
            r#"
            <html><body><main>
                <div class="header"><h2 class="chapter-title">Table of Contents</h2></div>
                <div id="maia-main" role="main">
                    <div id="content">
                        <h1 class="heading">Table of Contents</h1>
                        <ul><li><a href="/sre-book/introduction/">Chapter 1 - Introduction</a></li></ul>
                    </div>
                </div>
                <div class="footer">Copyright Google</div>
            </main></body></html>
            "#,
        );
        let url = Url::parse("https://sre.google/sre-book/table-of-contents/").unwrap();
        let extraction = extract_with_site_rule(
            &document,
            Some(&url),
            &ReadabilityOptions::default(),
            &Metadata::default(),
        )
        .unwrap()
        .unwrap();

        assert_eq!(extraction.diagnostic.name, "google-sre");
        assert!(extraction.attempt.content.contains("Chapter 1 - Introduction"));
        assert!(!extraction.attempt.content.contains("Copyright Google"));
    }

    #[test]
    fn disabled_cleanup_profile_still_absolutizes_urls() {
        let document = kuchiki::parse_html().one(
            r#"
            <html><body>
                <main>
                    <p>Article text with a <a href="/wiki/Hermitian_matrix">relative link</a>.</p>
                    <img src="/static/math.svg" alt="matrix">
                </main>
            </body></html>
            "#,
        );
        let url = Url::parse("https://en.wikipedia.org/wiki/Hermitian_matrix").unwrap();
        let options = ReadabilityOptions {
            site_profiles: vec![
                r#"
                name = "relative urls"
                hosts = ["wikipedia.org"]
                subdomains = true
                content_roots = ["main"]

                [cleanup]
                enabled = false
                prune = false
                "#
                .to_string(),
            ],
            ..Default::default()
        };
        let extraction = extract_with_site_rule(&document, Some(&url), &options, &Metadata::default())
            .unwrap()
            .unwrap();

        assert!(
            extraction
                .attempt
                .content
                .contains(r#"href="https://en.wikipedia.org/wiki/Hermitian_matrix""#),
            "{}",
            extraction.attempt.content
        );
        assert!(
            extraction
                .attempt
                .content
                .contains(r#"src="https://en.wikipedia.org/static/math.svg""#),
            "{}",
            extraction.attempt.content
        );
    }

    #[test]
    fn hacker_news_listing_uses_code_extractor() {
        let document = kuchiki::parse_html().one(
            r#"
            <html><head><title>Hacker News</title></head><body>
                <table><tbody>
                    <tr class="athing submission" id="1">
                        <td class="title"><span class="rank">1.</span></td>
                        <td class="title"><span class="titleline"><a href="https://example.com/a">First story</a><span class="sitebit comhead"> (<span class="sitestr">example.com</span>)</span></span></td>
                    </tr>
                    <tr><td></td><td class="subtext"><span class="score">12 points</span> by <a class="hnuser">alice</a> <a href="item?id=1">3 comments</a></td></tr>
                    <tr class="athing submission" id="2">
                        <td class="title"><span class="rank">2.</span></td>
                        <td class="title"><span class="titleline"><a href="https://example.com/b">Second story</a></span></td>
                    </tr>
                    <tr><td></td><td class="subtext"><span class="score">5 points</span> by <a class="hnuser">bob</a> <a href="item?id=2">discuss</a></td></tr>
                </tbody></table>
            </body></html>
            "#,
        );
        let url = Url::parse("https://news.ycombinator.com/news").unwrap();
        let extraction = extract_with_site_rule(
            &document,
            Some(&url),
            &ReadabilityOptions::default(),
            &Metadata::default(),
        )
        .unwrap()
        .unwrap();

        assert_eq!(extraction.diagnostic.source, SiteRuleSource::CodeExtractor);
        assert_eq!(extraction.diagnostic.name, "hacker-news");
        assert!(extraction.attempt.content.contains("<ol>"));
        assert!(extraction.attempt.content.contains("First story"));
        assert!(extraction.attempt.content.contains("3 comments"));
        assert!(!extraction.attempt.content.contains("<table"));
    }
}
