use std::collections::HashMap;

use kuchiki::NodeRef;
use kuchiki::traits::TendrilSink;
use scraper::Html;
use url::Url;

use crate::shared;

use super::config::{Article, ExtractFlags, ReadabilityOptions};
use super::diagnostics::{
    AttemptDiagnostic, CandidateDiagnostic, CandidateSelection, CleanupDiagnostic, ContentSelectorDiagnostic,
    ExtractionDiagnostics, ExtractionOutcome, ExtractionReport, FlagDiagnostic, NodeDiagnostic, RecoveryDiagnostic,
    SiteRuleSource,
};
use super::error::{Error, Result};
use super::regexes::RegexPattern;
use super::{cleanup, dom, json_schema, markdown, metadata, normalize, patterns, recovery, rules, scoring, serialize};
use super::{metadata::Metadata, scoring::Candidate};

const KNOWN_CONTENT_SELECTORS: &[&str] = &[
    "#article-body",
    "[itemprop='articleBody']",
    ".article-body",
    ".article__body",
    ".article-content",
    ".article__content",
    ".entry-content",
    ".post-content",
    ".content-wrapper",
];
const USEFUL_WORD_THRESHOLD: usize = 180;
const SUSPICIOUS_SIGNAL_RATIO: usize = 3;

pub struct ExtractAttempt {
    pub metadata: Metadata,
    pub content: String,
    pub text_content: String,
    pub text_len: usize,
}

impl ExtractAttempt {
    fn into_article(mut self) -> Article {
        if title_duplicates_site_name(&self.metadata)
            && let Some(heading) = first_content_heading(&self.content)
        {
            self.metadata.title = Some(heading);
        }

        if self.metadata.excerpt.as_deref().unwrap_or_default().trim().is_empty() {
            self.metadata.excerpt = metadata::first_paragraph_excerpt(&self.content);
        }

        Article {
            title: self.metadata.title,
            byline: self.metadata.byline,
            dir: self.metadata.dir,
            lang: self.metadata.lang,
            markdown: markdown::html_to_markdown(&self.content),
            content: self.content,
            text_content: self.text_content,
            length: self.text_len,
            excerpt: self.metadata.excerpt,
            site_name: self.metadata.site_name,
            published_time: self.metadata.published_time,
            image: self.metadata.image,
            domain: self.metadata.domain,
            favicon: self.metadata.favicon,
        }
    }
}

struct GrabDiagnostics {
    attempt: AttemptDiagnostic,
    content_selector: Option<ContentSelectorDiagnostic>,
}

impl From<ExtractFlags> for FlagDiagnostic {
    fn from(flags: ExtractFlags) -> Self {
        Self {
            strip_unlikely: flags.strip_unlikely,
            weight_classes: flags.weight_classes,
            clean_conditionally: flags.clean_conditionally,
        }
    }
}

struct EntryPointCandidate {
    node: NodeRef,
    score: f64,
    diagnostic: CandidateDiagnostic,
}

/// Extract a readable article from an HTML document.
///
/// `base_url` is optional. Pass it when the document contains relative links,
/// images, or metadata URLs. The function returns `Ok(None)` when the document
/// parses but no useful article content is found.
pub fn extract(html: &str, base_url: Option<&str>, options: &ReadabilityOptions) -> Result<Option<Article>> {
    Ok(extract_with_diagnostics(html, base_url, options)?.article)
}

/// Extract and return only the cleaned article HTML.
///
/// This is a convenience wrapper around [`extract`].
pub fn clean_article_html(html: &str, base_url: Option<&str>, options: &ReadabilityOptions) -> Result<Option<String>> {
    Ok(extract(html, base_url, options)?.map(|article| article.content))
}

/// Extract an article and include diagnostics for root selection and cleanup.
///
/// Diagnostics are intended for fixture work, site-profile tuning, and bug
/// reports. Most application code should call [`extract`].
pub fn extract_with_diagnostics(
    html: &str, base_url: Option<&str>, options: &ReadabilityOptions,
) -> Result<ExtractionReport> {
    let (working_html, source_recovery) = recovery::recover_html_snapshot(html);
    let html = working_html.as_str();
    let base_url = base_url
        .map(|base_url| Url::parse(base_url).map_err(|_| Error::InvalidBaseUrl(base_url.to_string())))
        .transpose()?;

    let document = Html::parse_document(html);
    enforce_element_limit(&document, options.max_elems_to_parse)?;
    let base_url = effective_base_url(&document, base_url.as_ref());

    let metadata = metadata::extract_metadata(&document, html, options, base_url.as_ref());
    let extraction_html = strip_raw_script_blocks(html);
    let mut best_attempt: Option<ExtractAttempt> = None;
    let mut diagnostics = ExtractionDiagnostics::default();

    if let Some((mut attempt, attempt_diagnostic)) = schema_text_attempt(&metadata, options, base_url.as_ref())? {
        attempt.metadata = metadata;
        diagnostics.selected_attempt = Some(0);
        diagnostics.outcome = ExtractionOutcome::Accepted;
        diagnostics.attempts.push(attempt_diagnostic);
        return Ok(ExtractionReport { article: Some(attempt.into_article()), diagnostics });
    }

    if options.content_selector.is_none()
        && let Some((mut attempt, attempt_diagnostic)) =
            known_content_attempt(&extraction_html, options, base_url.as_ref(), &metadata)?
    {
        attempt.metadata = metadata;
        diagnostics.selected_attempt = Some(0);
        diagnostics.outcome = ExtractionOutcome::Accepted;
        diagnostics.attempts.push(attempt_diagnostic);
        return Ok(ExtractionReport { article: Some(attempt.into_article()), diagnostics });
    }

    if let Some(mut rule_extraction) = try_site_rule(html, options, base_url.as_ref(), &metadata)?
        && rule_extraction.attempt.text_len > 0
    {
        let attempt_metadata = rule_extraction.attempt.metadata.clone();
        rule_extraction.attempt = json_schema::apply_schema_fallback(
            html,
            rule_extraction.attempt,
            &attempt_metadata,
            options,
            rule_extraction.flags,
            base_url.as_ref(),
        )?;
        rule_extraction.attempt.metadata = attempt_metadata;
        rule_extraction.diagnostic.text_len = rule_extraction.attempt.text_len;
        rule_extraction.diagnostic.accepted =
            matches!(rule_extraction.diagnostic.source, SiteRuleSource::CodeExtractor)
                || rule_extraction.attempt.text_len >= options.char_threshold;
        if rule_extraction.diagnostic.accepted {
            diagnostics.site_rule = Some(rule_extraction.diagnostic);
            diagnostics.outcome = ExtractionOutcome::Accepted;
            return Ok(ExtractionReport { article: Some(rule_extraction.attempt.into_article()), diagnostics });
        }
        rule_extraction.diagnostic.fallback_reason = Some(format!(
            "site rule text_len {} below char_threshold {}",
            rule_extraction.attempt.text_len, options.char_threshold
        ));
        diagnostics.site_rule = Some(rule_extraction.diagnostic);
        best_attempt = Some(rule_extraction.attempt);
    }

    let attempts = [
        ExtractFlags::all(),
        ExtractFlags { strip_unlikely: false, weight_classes: true, clean_conditionally: true },
        ExtractFlags { strip_unlikely: false, weight_classes: false, clean_conditionally: true },
        ExtractFlags { strip_unlikely: false, weight_classes: false, clean_conditionally: false },
    ];

    for (index, flags) in attempts.into_iter().enumerate() {
        let dom = kuchiki::parse_html().one(extraction_html.as_ref());
        let mut recovery = prep_document(&dom, options, flags);
        recovery.shadow_roots_flattened += source_recovery.shadow_roots_flattened;

        let Some((mut attempt, attempt_diagnostic)) = grab_article(
            &dom,
            options,
            flags,
            index,
            recovery.clone(),
            base_url.as_ref(),
            &metadata,
        )?
        else {
            diagnostics.attempts.push(AttemptDiagnostic {
                index,
                flags: flags.into(),
                candidate_count: 0,
                candidates: Vec::new(),
                entry_points: Vec::new(),
                selected_root: None,
                cleanup: None,
                recovery,
                text_len: 0,
                accepted: false,
            });
            continue;
        };

        if diagnostics.content_selector.is_none() {
            diagnostics.content_selector = attempt_diagnostic.content_selector.clone();
        }
        diagnostics.attempts.push(attempt_diagnostic.attempt);
        let diagnostic_index = diagnostics.attempts.len() - 1;

        if attempt.text_len >= options.char_threshold
            && !should_retry_short_or_suspicious(&attempt, &diagnostics.attempts[diagnostic_index], flags, options)
        {
            attempt = json_schema::apply_schema_fallback(html, attempt, &metadata, options, flags, base_url.as_ref())?;
            attempt.metadata = metadata;
            diagnostics.selected_attempt = Some(diagnostic_index);
            diagnostics.outcome = ExtractionOutcome::Accepted;
            return Ok(ExtractionReport { article: Some(attempt.into_article()), diagnostics });
        }

        if best_attempt
            .as_ref()
            .map(|best| attempt.text_len > best.text_len)
            .unwrap_or(true)
        {
            diagnostics.selected_attempt = Some(diagnostic_index);
            best_attempt = Some(attempt);
        }
    }

    let Some(mut attempt) = best_attempt.filter(|attempt| attempt.text_len > 0) else {
        diagnostics.outcome = ExtractionOutcome::NoContent;
        return Ok(ExtractionReport { article: None, diagnostics });
    };
    attempt = json_schema::apply_schema_fallback(
        html,
        attempt,
        &metadata,
        options,
        ExtractFlags { strip_unlikely: false, weight_classes: false, clean_conditionally: false },
        base_url.as_ref(),
    )?;
    attempt.metadata = metadata;
    diagnostics.outcome = ExtractionOutcome::BestAttempt;
    Ok(ExtractionReport { article: Some(attempt.into_article()), diagnostics })
}

pub fn prep_document(document: &NodeRef, options: &ReadabilityOptions, flags: ExtractFlags) -> RecoveryDiagnostic {
    let recovery = recovery::recover(document, options.mobile_viewport_width);
    unwrap_noscript_images(document);
    dom::remove_matching(document, "script, style");
    normalize_markup(document);

    if flags.strip_unlikely {
        let nodes = dom::select_nodes(document, "*");
        for node in nodes {
            if !dom::is_kuchiki_visible(&node) || dom::has_unlikely_role(&node) {
                node.detach();
                continue;
            }

            let tag = dom::node_name(&node);
            if tag == "body" || tag == "a" {
                continue;
            }

            let match_string = dom::class_id_string(&node);
            if RegexPattern::UnlikelyCandidates.to_regex().is_match(&match_string)
                && !RegexPattern::MaybeCandidate.to_regex().is_match(&match_string)
                && !dom::has_ancestor_tag(&node, "table", 3)
                && !dom::has_ancestor_tag(&node, "code", 3)
            {
                node.detach();
            }
        }
    } else {
        let nodes = dom::select_nodes(document, "*");
        for node in nodes {
            if !dom::is_kuchiki_visible(&node) {
                node.detach();
            }
        }
    }
    recovery
}

pub fn serialize_roots(
    roots: Vec<NodeRef>, opts: &ReadabilityOptions, flags: ExtractFlags, base_url: Option<&Url>, metadata: &Metadata,
) -> Result<(ExtractAttempt, CleanupDiagnostic)> {
    let text_len_before = serialize::text_content(&roots).encode_utf16().count();
    let element_count_before = roots.iter().map(element_count).sum();
    let root_selectors = roots.iter().map(node_selector).collect();

    cleanup::cleanup_article(&roots, opts, flags, base_url, metadata);
    normalize::normalize_article(&roots, metadata.title.as_deref());
    let roots = cleanup::remove_trailing_chrome_roots(roots);

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
    let element_count_after = roots.iter().map(element_count).sum();

    let attempt = ExtractAttempt { metadata: Metadata::default(), content, text_content, text_len };
    let cleanup = CleanupDiagnostic {
        roots: root_selectors,
        text_len_before,
        text_len_after: text_len,
        element_count_before,
        element_count_after,
        removed_elements: element_count_before.saturating_sub(element_count_after),
    };

    Ok((attempt, cleanup))
}

pub fn element_count(node: &NodeRef) -> usize {
    node.descendants().filter(|node| node.as_element().is_some()).count()
}

fn strip_raw_script_blocks(html: &str) -> String {
    RegexPattern::RawScript.to_regex().replace_all(html, "").into_owned()
}

fn should_retry_short_or_suspicious(
    attempt: &ExtractAttempt, diagnostic: &AttemptDiagnostic, flags: ExtractFlags, options: &ReadabilityOptions,
) -> bool {
    flags != (ExtractFlags { strip_unlikely: false, weight_classes: false, clean_conditionally: false })
        && (short_after_unlikely_stripping(attempt, flags)
            || far_below_best_content_signal(attempt.text_len, diagnostic, options.char_threshold))
}

fn short_after_unlikely_stripping(attempt: &ExtractAttempt, flags: ExtractFlags) -> bool {
    flags.strip_unlikely && shared::word_count(&attempt.text_content) < USEFUL_WORD_THRESHOLD
}

fn far_below_best_content_signal(text_len: usize, diagnostic: &AttemptDiagnostic, char_threshold: usize) -> bool {
    best_content_signal_len(diagnostic) >= char_threshold.saturating_mul(2).max(1)
        && text_len.saturating_mul(SUSPICIOUS_SIGNAL_RATIO) < best_content_signal_len(diagnostic)
}

fn best_content_signal_len(diagnostic: &AttemptDiagnostic) -> usize {
    diagnostic
        .selected_root
        .iter()
        .chain(diagnostic.candidates.iter().map(|candidate| &candidate.node))
        .chain(diagnostic.entry_points.iter().map(|entry_point| &entry_point.node))
        .map(|node| node.text_len)
        .max()
        .unwrap_or(0)
}

fn known_content_attempt(
    html: &str, opts: &ReadabilityOptions, base_url: Option<&Url>, metadata: &Metadata,
) -> Result<Option<(ExtractAttempt, AttemptDiagnostic)>> {
    let document = kuchiki::parse_html().one(html);
    let flags = ExtractFlags { strip_unlikely: false, weight_classes: false, clean_conditionally: false };

    for selector in KNOWN_CONTENT_SELECTORS {
        let roots = dom::select_nodes(&document, selector);
        for root in roots {
            let recovery = recovery::recover(&root, opts.mobile_viewport_width);
            unwrap_noscript_images(&root);
            dom::remove_matching(&root, "script, style");
            normalize_markup(&root);
            let selected_root = node_diagnostic(&root);
            let (attempt, cleanup) = serialize_roots(vec![root], opts, flags, base_url, metadata)?;
            if attempt.text_len < opts.char_threshold {
                continue;
            }

            let diagnostic = AttemptDiagnostic {
                index: 0,
                flags: flags.into(),
                candidate_count: 0,
                candidates: Vec::new(),
                entry_points: Vec::new(),
                selected_root: Some(selected_root),
                cleanup: Some(cleanup),
                recovery,
                text_len: attempt.text_len,
                accepted: true,
            };
            return Ok(Some((attempt, diagnostic)));
        }
    }

    Ok(None)
}

fn schema_text_attempt(
    metadata: &Metadata, opts: &ReadabilityOptions, base_url: Option<&Url>,
) -> Result<Option<(ExtractAttempt, AttemptDiagnostic)>> {
    let Some(schema_text) = metadata.schema_text.as_deref() else {
        return Ok(None);
    };

    let normalized = patterns::normalize_spaces(schema_text.trim());
    if normalized.encode_utf16().count() < opts.char_threshold {
        return Ok(None);
    }

    let escaped = shared::escape_html(&normalized);
    let document = kuchiki::parse_html().one(format!("<html><body><article><p>{escaped}</p></article></body></html>"));
    let Some(root) = dom::select_nodes(&document, "article").into_iter().next() else {
        return Ok(None);
    };

    let selected_root = node_diagnostic(&root);
    let flags = ExtractFlags { strip_unlikely: false, weight_classes: false, clean_conditionally: false };
    let (attempt, cleanup) = serialize_roots(vec![root], opts, flags, base_url, metadata)?;
    if attempt.text_len < opts.char_threshold {
        return Ok(None);
    }

    let diagnostic = AttemptDiagnostic {
        index: 0,
        flags: flags.into(),
        candidate_count: 0,
        candidates: Vec::new(),
        entry_points: Vec::new(),
        selected_root: Some(selected_root),
        cleanup: Some(cleanup),
        recovery: RecoveryDiagnostic::default(),
        text_len: attempt.text_len,
        accepted: true,
    };

    Ok(Some((attempt, diagnostic)))
}

fn title_duplicates_site_name(metadata: &Metadata) -> bool {
    let title = metadata.title.as_deref().unwrap_or_default().trim();
    let site_name = metadata.site_name.as_deref().unwrap_or_default().trim();
    !title.is_empty() && title.eq_ignore_ascii_case(site_name)
}

fn first_content_heading(content: &str) -> Option<String> {
    let document = kuchiki::parse_html().one(format!("<html><body>{content}</body></html>"));
    dom::select_nodes(&document, "h1, h2, h3")
        .into_iter()
        .map(|heading| patterns::normalize_spaces(dom::inner_text(&heading).trim()))
        .find(|heading| !heading.is_empty())
}

fn try_site_rule(
    html: &str, options: &ReadabilityOptions, base_url: Option<&Url>, metadata: &Metadata,
) -> Result<Option<rules::RuleExtraction>> {
    let doc = kuchiki::parse_html().one(html);
    prep_document(
        &doc,
        options,
        ExtractFlags { strip_unlikely: false, weight_classes: false, clean_conditionally: false },
    );
    rules::extract_with_site_rule(&doc, base_url, options, metadata)
}

fn normalize_markup(document: &NodeRef) {
    for font in dom::select_nodes(document, "font") {
        let _ = dom::retag_node(&font, "span");
    }

    for div in dom::select_nodes(document, "div") {
        if has_single_element_child(&div, "p") && direct_text_is_empty(&div) {
            dom::replace_with_children(&div);
        }
    }

    for div in dom::select_nodes(document, "div") {
        if !has_child_block_element(&div) && !dom::inner_text(&div).is_empty() {
            let _ = dom::retag_node(&div, "p");
        }
    }
}

fn has_single_element_child(node: &NodeRef, tag: &str) -> bool {
    let mut element_children = node.children().filter(|child| child.as_element().is_some());
    let Some(first) = element_children.next() else {
        return false;
    };
    element_children.next().is_none() && dom::node_name(&first) == tag
}

fn direct_text_is_empty(node: &NodeRef) -> bool {
    node.children()
        .filter_map(|child| child.as_text().map(|text| text.borrow().to_string()))
        .all(|text| text.trim().is_empty())
}

fn has_child_block_element(node: &NodeRef) -> bool {
    !dom::select_nodes(
        node,
        "address, article, aside, blockquote, canvas, dd, div, dl, dt, fieldset, figcaption, figure, footer, form, h1, h2, h3, h4, h5, h6, header, hgroup, hr, li, main, nav, noscript, ol, output, p, pre, section, table, tfoot, ul, video",
    )
    .is_empty()
}

fn effective_base_url(document: &Html, base_url: Option<&Url>) -> Option<Url> {
    let base_url = base_url.cloned()?;
    let selector = patterns::selector("base[href]");
    document
        .select(&selector)
        .next()
        .and_then(|base| base.value().attr("href"))
        .and_then(|href| base_url.join(href).ok())
        .or(Some(base_url))
}

fn unwrap_noscript_images(document: &NodeRef) {
    for noscript in dom::select_nodes(document, "noscript") {
        let content = unescape_basic_html(&noscript.text_contents());
        let lower_content = content.to_ascii_lowercase();
        if !lower_content.contains("<img") && !lower_content.contains("<picture") {
            continue;
        }

        let fragment = kuchiki::parse_html().one(format!("<html><body>{content}</body></html>"));
        let Some(body) = dom::select_nodes(&fragment, "body").into_iter().next() else {
            continue;
        };
        let children: Vec<_> = body.children().collect();
        if children.is_empty() {
            continue;
        }

        for child in children {
            noscript.insert_before(child);
        }
        noscript.detach();
    }
}

// TODO: should this be in this mod?
fn unescape_basic_html(value: &str) -> String {
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#34;", "\"")
        .replace("&amp;", "&")
}

fn grab_article(
    doc: &NodeRef, opts: &ReadabilityOptions, flags: ExtractFlags, index: usize, recovery: RecoveryDiagnostic,
    base_url: Option<&Url>, metadata: &Metadata,
) -> Result<Option<(ExtractAttempt, GrabDiagnostics)>> {
    if let Some(selector) = opts.content_selector.as_deref()
        && let Some(root) = dom::select_nodes(doc, selector).into_iter().next()
    {
        let selector_diagnostic = ContentSelectorDiagnostic {
            selector: selector.to_string(),
            matched: true,
            selected: Some(node_diagnostic(&root)),
        };
        let (attempt, cleanup) = serialize_roots(vec![root], opts, flags, base_url, metadata)?;
        let attempt_diagnostic = AttemptDiagnostic {
            index,
            flags: flags.into(),
            candidate_count: 0,
            candidates: Vec::new(),
            entry_points: Vec::new(),
            selected_root: selector_diagnostic.selected.clone(),
            cleanup: Some(cleanup),
            recovery,
            text_len: attempt.text_len,
            accepted: attempt.text_len >= opts.char_threshold,
        };
        return Ok(Some((
            attempt,
            GrabDiagnostics { attempt: attempt_diagnostic, content_selector: Some(selector_diagnostic) },
        )));
    }

    let entry_points = entry_point_candidates(doc);
    let mut candidates = scoring::score_candidates(doc, flags);
    if candidates.is_empty() {
        let body = dom::select_nodes(doc, "body").into_iter().next();
        if let Some(body) = body {
            candidates.push(Candidate { node: body, score: 1.0 });
        }
    }

    if candidates.is_empty() {
        return Ok(None);
    }

    for candidate in &mut candidates {
        candidate.score *= 1.0 - scoring::link_density(&candidate.node);
    }

    let max_candidate_score = candidates
        .iter()
        .map(|candidate| candidate.score)
        .max_by(|a, b| a.total_cmp(b))
        .unwrap_or(0.0);
    let selected_entry_id = selected_entry_point(&entry_points).map(|entry_point| {
        let id = dom::node_id(&entry_point.node);
        let preferred_score = entry_point.score.max(max_candidate_score + 25.0);
        if let Some(candidate) = candidates
            .iter_mut()
            .find(|candidate| dom::node_id(&candidate.node) == id)
        {
            candidate.score = candidate.score.max(preferred_score);
        } else {
            candidates.push(Candidate { node: entry_point.node.clone(), score: preferred_score });
        }
        id
    });

    candidates.sort_by(|a, b| b.score.total_cmp(&a.score));

    let candidate_count = candidates.len();

    candidates.truncate(opts.nb_top_candidates.max(1));

    let candidate_diagnostics: Vec<_> = candidates
        .iter()
        .map(|candidate| CandidateDiagnostic {
            node: node_diagnostic(&candidate.node),
            score: round_score(candidate.score),
            selected_by: if selected_entry_id == Some(dom::node_id(&candidate.node)) {
                CandidateSelection::EntryPointPreselection
            } else {
                CandidateSelection::CandidateScoring
            },
        })
        .collect();

    let top_candidate = candidates[0].node.clone();
    let top_score = candidates[0].score;
    let top_id = dom::node_id(&top_candidate);
    let score_by_id: HashMap<usize, f64> = candidates
        .iter()
        .map(|candidate| (dom::node_id(&candidate.node), candidate.score))
        .collect();

    let parent = top_candidate.parent().unwrap_or_else(|| doc.clone());
    let sibling_threshold = 10.0_f64.max(top_score * 0.2);
    let top_class = dom::attr(&top_candidate, "class").unwrap_or_default();

    let mut included = Vec::new();
    for sibling in parent.children().filter(|node| node.as_element().is_some()) {
        let mut append = dom::node_id(&sibling) == top_id;

        if !append {
            let mut content_bonus = 0.0;
            if !top_class.is_empty() && dom::attr(&sibling, "class").as_deref() == Some(top_class.as_str()) {
                content_bonus += top_score * 0.2;
            }

            if score_by_id.get(&dom::node_id(&sibling)).copied().unwrap_or(0.0) + content_bonus >= sibling_threshold {
                append = true;
            } else if dom::node_name(&sibling) == "p" {
                let density = scoring::link_density(&sibling);
                let text = dom::inner_text(&sibling);
                let len = text.chars().count();
                if (len > 80 && density < 0.25) || (len < 80 && len > 0 && density == 0.0 && text.contains(". ")) {
                    append = true;
                }
            }
        }

        if append {
            included.push(sibling);
        }
    }

    if included.is_empty() {
        included.push(top_candidate);
    }

    let selected_root = included.first().map(node_diagnostic);
    let (attempt, cleanup) = serialize_roots(included, opts, flags, base_url, metadata)?;
    let content_selector = opts
        .content_selector
        .as_ref()
        .map(|selector| ContentSelectorDiagnostic { selector: selector.clone(), matched: false, selected: None });
    let attempt_diagnostic = AttemptDiagnostic {
        index,
        flags: flags.into(),
        candidate_count,
        candidates: candidate_diagnostics,
        entry_points: entry_points
            .iter()
            .map(|entry_point| entry_point.diagnostic.clone())
            .collect(),
        selected_root,
        cleanup: Some(cleanup),
        recovery,
        text_len: attempt.text_len,
        accepted: attempt.text_len >= opts.char_threshold,
    };

    Ok(Some((
        attempt,
        GrabDiagnostics { attempt: attempt_diagnostic, content_selector },
    )))
}

fn entry_point_candidates(document: &NodeRef) -> Vec<EntryPointCandidate> {
    // TODO: could this be a constant?
    let selectors = [
        "article",
        "main",
        r#"[role="main"]"#,
        "#content",
        "#main",
        "#article",
        ".content",
        ".main",
        ".article",
        ".post",
        ".entry-content",
    ];
    let mut candidates = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for selector in selectors {
        for node in dom::select_nodes(document, selector) {
            if !seen.insert(dom::node_id(&node)) {
                continue;
            }
            let text_len = dom::inner_text(&node).chars().count();
            if text_len < 80 {
                continue;
            }
            let link_density = scoring::link_density(&node);
            if link_density > 0.65 {
                continue;
            }
            let score = (text_len as f64 / 25.0) * (1.0 - link_density).max(0.0)
                + scoring::class_weight(&node, ExtractFlags::all()) as f64;
            let diagnostic = CandidateDiagnostic {
                node: node_diagnostic(&node),
                score: round_score(score),
                selected_by: CandidateSelection::EntryPointPreselection,
            };
            candidates.push(EntryPointCandidate { node, score, diagnostic });
        }
    }

    candidates.sort_by(|a, b| b.score.total_cmp(&a.score));
    candidates.truncate(8);
    candidates
}

fn selected_entry_point(entry_points: &[EntryPointCandidate]) -> Option<Candidate> {
    let best = entry_points.first()?;
    if best.diagnostic.node.text_len < 140 || best.score < 8.0 {
        return None;
    }
    Some(Candidate { node: best.node.clone(), score: best.score + 25.0 })
}

fn node_diagnostic(node: &NodeRef) -> NodeDiagnostic {
    let class = dom::attr(node, "class").unwrap_or_default();
    NodeDiagnostic {
        selector: node_selector(node),
        tag: dom::node_name(node),
        id: dom::attr(node, "id"),
        classes: class.split_whitespace().map(str::to_string).collect(),
        text_len: dom::inner_text(node).encode_utf16().count(),
        link_density: round_score(scoring::link_density(node)),
    }
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

fn round_score(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn enforce_element_limit(document: &Html, limit: Option<usize>) -> Result<()> {
    let Some(limit) = limit else {
        return Ok(());
    };

    let selector = patterns::selector("*");
    let actual = document.select(&selector).count();
    if actual > limit {
        return Err(Error::MaxElemsExceeded { actual, limit });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MediaRetention;
    use crate::patterns::normalize_spaces;

    #[test]
    fn returns_article_for_simple_document() {
        let article = extract(
            "<html><head><title>Example Article</title></head><body><article><p>This is a long enough paragraph, with punctuation, to become a readable article body for the MVP extractor.</p></article></body></html>",
            None,
            &ReadabilityOptions { char_threshold: 0, ..Default::default() },
        )
        .unwrap()
        .unwrap();

        assert_eq!(article.title.as_deref(), Some("Example Article"));
        assert!(article.content.contains("readability-page-1"));
        assert!(article.text_content.contains("long enough paragraph"));
        assert!(article.length > 25);
    }

    #[test]
    fn accepts_long_json_ld_article_body_before_candidate_scoring() {
        let schema_text = "This article body comes from JSON-LD before the generic scoring path runs. ".repeat(20);
        let html = format!(
            r#"
            <html><head>
                <title>Schema Article</title>
                <script type="application/ld+json">
                    {{"@type":"NewsArticle","headline":"Schema Article","articleBody":"{schema_text}"}}
                </script>
            </head><body>
                <main>
                    <div class="feed">
                        <p>Navigation, recommendations, and page chrome should not win extraction.</p>
                    </div>
                </main>
            </body></html>
            "#
        );

        let report = extract_with_diagnostics(&html, None, &ReadabilityOptions::default()).unwrap();
        let article = report.article.unwrap();

        assert_eq!(report.diagnostics.outcome, ExtractionOutcome::Accepted);
        assert_eq!(report.diagnostics.selected_attempt, Some(0));
        assert_eq!(report.diagnostics.attempts.len(), 1);
        assert_eq!(report.diagnostics.attempts[0].candidate_count, 0);
        assert!(article.text_content.contains("JSON-LD before the generic scoring path"));
        assert!(!article.text_content.contains("Navigation, recommendations"));
    }

    #[test]
    fn accepts_known_article_body_container_before_candidate_scoring() {
        let body = (0..12)
            .map(|index| {
                format!(
                    "<p>This focused article-body container paragraph {index} has useful prose, \
                    concrete detail, and enough punctuation to be accepted before scoring.</p>"
                )
            })
            .collect::<String>();
        let html = format!(
            r#"
            <html><head><title>Known Container</title></head><body>
                <div class="navigation">
                    <p>Navigation, recommendations, and other page chrome.</p>
                </div>
                <div id="article-body" class="text-copy">
                    {body}
                </div>
            </body></html>
            "#
        );

        let report = extract_with_diagnostics(&html, None, &ReadabilityOptions::default()).unwrap();
        let article = report.article.unwrap();

        assert_eq!(report.diagnostics.outcome, ExtractionOutcome::Accepted);
        assert_eq!(report.diagnostics.selected_attempt, Some(0));
        assert_eq!(report.diagnostics.attempts.len(), 1);
        assert_eq!(report.diagnostics.attempts[0].candidate_count, 0);
        assert!(article.text_content.contains("focused article-body container"));
        assert!(!article.text_content.contains("Navigation, recommendations"));
    }

    #[test]
    fn reports_invalid_base_url_and_element_limit() {
        let invalid_url = extract(
            "<html><body><p>text</p></body></html>",
            Some("not a url"),
            &Default::default(),
        );
        assert!(matches!(invalid_url, Err(Error::InvalidBaseUrl(_))));

        let too_many_elements = extract(
            "<html><body><main><p>text</p></main></body></html>",
            None,
            &ReadabilityOptions { max_elems_to_parse: Some(2), ..Default::default() },
        );
        assert!(matches!(too_many_elements, Err(Error::MaxElemsExceeded { .. })));
    }

    #[test]
    fn honors_base_element_for_relative_urls() {
        let fixture = lectito_fixtures::load_fixture("base-url-base-element").unwrap();
        let article = extract(
            &fixture.source,
            Some("http://fakehost/test/page.html"),
            &ReadabilityOptions { char_threshold: 0, ..Default::default() },
        )
        .unwrap()
        .unwrap();

        assert!(article.content.contains(r#"href="http://fakehost/foo/bar/baz.html""#));
        assert!(article.content.contains(r#"src="http://fakehost/foo/bar/baz.png""#));
        assert!(
            !article
                .content
                .contains(r#"href="http://fakehost/test/foo/bar/baz.html""#)
        );
    }

    #[test]
    fn repairs_noscript_images_and_scores_br_divs() {
        let noscript_article = extract(
            r#"<html><body><article><p>Enough text, with punctuation, to choose the article body for this regression.</p><noscript>&lt;img src="/image.jpg" alt="fallback"&gt;</noscript></article></body></html>"#,
            Some("https://example.com/story"),
            &ReadabilityOptions { char_threshold: 0, ..Default::default() },
        )
        .unwrap()
        .unwrap();
        assert!(
            noscript_article
                .content
                .contains(r#"src="https://example.com/image.jpg""#)
        );

        let br_article = extract(
            "<html><body><div>First long line with enough words to score well.<br><br>Second long line, also with enough words and punctuation to survive extraction.</div></body></html>",
            None,
            &ReadabilityOptions { char_threshold: 0, ..Default::default() },
        )
        .unwrap()
        .unwrap();
        assert!(br_article.text_content.contains("Second long line"));
    }

    #[test]
    fn content_selector_override_forces_root_and_reports_diagnostics() {
        let report = extract_with_diagnostics(
            r#"
            <html><body>
                <main>
                    <p>Navigation teaser text that should lose when the explicit selector points elsewhere.</p>
                </main>
                <section id="forced">
                    <p>This forced article body has enough punctuation, enough words, and enough detail to become the returned content.</p>
                </section>
            </body></html>
            "#,
            None,
            &ReadabilityOptions {
                char_threshold: 0,
                content_selector: Some("#forced".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        let article = report.article.unwrap();
        assert!(article.text_content.contains("forced article body"));
        assert!(!article.text_content.contains("Navigation teaser"));

        let selector = report.diagnostics.content_selector.unwrap();
        assert_eq!(selector.selector, "#forced");
        assert!(selector.matched);
        assert_eq!(selector.selected.unwrap().selector, "section#forced");
        assert!(report.diagnostics.attempts[0].cleanup.is_some());
    }

    #[test]
    fn custom_site_profile_can_select_content_root() {
        let profile = r##"
            name = "example profile"
            hosts = ["example.com"]
            content_roots = ["#profiled"]
            remove = [".ad"]

            [metadata]
            title = ["h1"]
            site_name = "Example"
        "##;
        let report = extract_with_diagnostics(
            r#"
            <html><body>
                <main><p>Generic main content that should not be returned.</p></main>
                <article id="profiled">
                    <h1>Profiled Story</h1>
                    <p>This profile-selected article body has enough punctuation, enough words, and enough detail to become the returned content.</p>
                    <p class="ad">Sponsored interruption.</p>
                </article>
            </body></html>
            "#,
            Some("https://example.com/story"),
            &ReadabilityOptions {
                char_threshold: 0,
                site_profiles: vec![profile.to_string()],
                ..Default::default()
            },
        )
        .unwrap();

        let article = report.article.unwrap();
        assert_eq!(article.title.as_deref(), Some("Profiled Story"));
        assert_eq!(article.site_name.as_deref(), Some("Example"));
        assert!(article.text_content.contains("profile-selected article body"));
        assert!(!article.text_content.contains("Generic main content"));
        assert!(!article.text_content.contains("Sponsored interruption"));

        let diagnostic = report.diagnostics.site_rule.unwrap();
        assert_eq!(diagnostic.name, "example profile");
        assert!(diagnostic.accepted);
        assert_eq!(diagnostic.roots, vec!["article#profiled"]);
    }

    #[test]
    fn preserves_link_heavy_article_lists() {
        let links = (0..30)
            .map(|index| format!(r#"<li><a href="https://example.com/api/{index}"><code>Api::{index}</code></a></li>"#))
            .collect::<String>();
        let html = format!(
            r#"<html><body><article><h1>Release notes</h1>
            <p>This release includes new capabilities and enough explanatory prose to be selected as an article candidate.</p>
            <p>The following APIs are now stable and should remain in the extracted content.</p>
            <h2>Stabilized APIs</h2><ul>{links}</ul>
            <p>Other changes include compiler fixes, cargo improvements, and documentation updates for users.</p>
            </article></body></html>"#
        );

        let article = extract(
            &html,
            Some("https://example.com/release"),
            &ReadabilityOptions { char_threshold: 0, ..Default::default() },
        )
        .unwrap()
        .unwrap();

        assert!(article.content.contains("Stabilized APIs"), "{}", article.content);
        assert!(article.content.contains("Api::0"), "{}", article.content);
        assert!(article.content.contains("Api::29"), "{}", article.content);
    }

    #[test]
    fn uses_content_heading_when_metadata_title_is_site_name() {
        let article = extract(
            r#"<html><head>
                <meta property="og:site_name" content="Example Site">
                <meta property="og:title" content="Example Site">
            </head><body>
                <h1>Example Site</h1>
                <article><h2>Short Post</h2><p>This article body has enough prose and punctuation to be selected as readable content.</p></article>
            </body></html>"#,
            Some("https://example.com/post"),
            &ReadabilityOptions { char_threshold: 0, ..Default::default() },
        )
        .unwrap()
        .unwrap();

        assert_eq!(article.title.as_deref(), Some("Short Post"));
    }

    #[test]
    fn article_media_retention_preserves_body_figures() {
        let html = r#"
            <html><body><article>
                <h1>Quality curves</h1>
                <p>This article explains a diagram, with enough punctuation and prose to be extracted as readable content.</p>
                <div class="figure" id="curve.png"><img src="curve.png"><p class="photoCaption"></p></div>
                <p>The paragraph after the figure continues the argument and proves the figure sits inside the article body.</p>
            </article></body></html>
        "#;

        let article = extract(
            html,
            Some("https://example.com/articles/story.html"),
            &ReadabilityOptions { char_threshold: 0, media_retention: MediaRetention::Article, ..Default::default() },
        )
        .unwrap()
        .unwrap();

        assert!(article.content.contains("<img"), "{}", article.content);
        assert!(
            article.content.contains("https://example.com/articles/curve.png"),
            "{}",
            article.content
        );
    }

    #[test]
    fn none_media_retention_removes_body_figures() {
        let html = r#"
            <html><body><article>
                <p>This article explains a diagram, with enough punctuation and prose to be extracted as readable content.</p>
                <div class="figure"><img src="curve.png"></div>
                <p>The paragraph after the figure continues the argument and proves the figure sits inside the article body.</p>
            </article></body></html>
        "#;

        let article = extract(
            html,
            Some("https://example.com/articles/story.html"),
            &ReadabilityOptions { char_threshold: 0, media_retention: MediaRetention::None, ..Default::default() },
        )
        .unwrap()
        .unwrap();

        assert!(!article.content.contains("<img"), "{}", article.content);
    }

    #[test]
    fn weak_site_profile_output_falls_back_to_generic_extraction() {
        let profile = r##"
            name = "weak profile"
            hosts = ["example.com"]
            content_roots = ["#short"]
        "##;
        let report = extract_with_diagnostics(
            r#"
            <html><body>
                <section id="short"><p>Too short.</p></section>
                <article>
                    <p>This generic article body is intentionally much longer than the profiled short node. It has enough punctuation, enough words, and enough detail to pass the configured threshold through normal readability scoring.</p>
                    <p>The second paragraph adds more meaningful article text so fallback clearly selects the generic article instead of the weak site profile.</p>
                </article>
            </body></html>
            "#,
            Some("https://example.com/story"),
            &ReadabilityOptions {
                char_threshold: 160,
                site_profiles: vec![profile.to_string()],
                ..Default::default()
            },
        )
        .unwrap();

        let article = report.article.unwrap();
        assert!(article.text_content.contains("generic article body"));
        assert!(!article.text_content.trim_start().starts_with("Too short."));

        let diagnostic = report.diagnostics.site_rule.unwrap();
        assert_eq!(diagnostic.name, "weak profile");
        assert!(!diagnostic.accepted);
        assert!(diagnostic.fallback_reason.unwrap().contains("below char_threshold"));
    }

    #[test]
    fn retries_relaxed_cleanup_when_result_is_far_below_content_signal() {
        let paragraphs = (0..12)
            .map(|index| {
                format!(
                    "<p>Recovered paragraph {index} has substantial article prose, commas, \
                    concrete detail, and enough punctuation to prove it belongs in the story.</p>"
                )
            })
            .collect::<String>();
        let html = format!(
            r#"
            <html><body>
                <article>
                    <p>This opening paragraph is long enough to pass the minimum threshold, but it is only the beginning of the article and should not be accepted alone.</p>
                    <section class="shopping">
                        {paragraphs}
                    </section>
                </article>
            </body></html>
            "#
        );

        let report = extract_with_diagnostics(&html, None, &ReadabilityOptions::default()).unwrap();
        let article = report.article.unwrap();

        assert_eq!(report.diagnostics.outcome, ExtractionOutcome::Accepted);
        assert_eq!(report.diagnostics.selected_attempt, Some(2));
        assert_eq!(report.diagnostics.attempts.len(), 3);
        assert!(article.text_content.contains("Recovered paragraph 11"));
    }

    #[test]
    fn retries_without_unlikely_stripping_when_first_result_is_short() {
        let paragraphs = (0..30)
            .map(|index| {
                format!(
                    "<p>Hidden article paragraph {index} has useful prose, commas, \
                    concrete examples, and enough detail to beat the short fallback result.</p>"
                )
            })
            .collect::<String>();
        let html = format!(
            r#"
            <html><body>
                <article>
                    <p>This short fallback paragraph has enough characters to pass the old threshold, but too few words to be a useful extraction.</p>
                    <p>The second short fallback paragraph adds characters while still leaving the extraction well under a useful article word count.</p>
                    <p>The third short fallback paragraph repeats the same problem: enough length for acceptance, not enough substance to stop retrying.</p>
                    <p>The fourth short fallback paragraph keeps the first attempt above the default character threshold without making it useful.</p>
                </article>
                <section class="comments">
                    {paragraphs}
                </section>
            </body></html>
            "#
        );

        let report = extract_with_diagnostics(&html, None, &ReadabilityOptions::default()).unwrap();
        let article = report.article.unwrap();

        assert_eq!(report.diagnostics.outcome, ExtractionOutcome::Accepted);
        assert_eq!(report.diagnostics.selected_attempt, Some(1));
        assert!(report.diagnostics.attempts.len() > 1);
        assert!(article.text_content.contains("Hidden article paragraph 9"));
    }

    #[test]
    fn code_extractor_output_accepts_short_specialized_pages() {
        let report = extract_with_diagnostics(
            r#"
            <html><head><title>Short Link | Hacker News</title></head><body>
                <table><tbody>
                    <tr class="athing" id="1">
                        <td><span class="titleline"><a href="https://example.com">Short Link</a></span></td>
                    </tr>
                    <tr><td class="subtext"><span class="score">1 point</span> by <a class="hnuser">alice</a> <span class="age" title="2026-05-22T00:00:00"><a href="item?id=1">1 hour ago</a></span></td></tr>
                    <tr class="fatitem"><td></td></tr>
                </tbody></table>
            </body></html>
            "#,
            Some("https://news.ycombinator.com/item?id=1"),
            &ReadabilityOptions::default(),
        )
        .unwrap();

        assert_eq!(report.diagnostics.outcome, ExtractionOutcome::Accepted);
        let diagnostic = report.diagnostics.site_rule.unwrap();
        assert_eq!(diagnostic.source, SiteRuleSource::CodeExtractor);
        assert!(diagnostic.accepted);
        assert!(report.article.unwrap().text_content.contains("Short Link"));
    }

    #[test]
    fn unmatched_content_selector_falls_back_to_scoring_and_reports_candidates() {
        let report = extract_with_diagnostics(
            r#"
            <html><body>
                <article class="story">
                    <p>This article body is long enough, punctuated enough, and clean enough to be selected by normal scoring.</p>
                </article>
            </body></html>
            "#,
            None,
            &ReadabilityOptions {
                char_threshold: 0,
                content_selector: Some(".missing".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        assert!(
            report
                .article
                .unwrap()
                .text_content
                .contains("selected by normal scoring")
        );
        let selector = report.diagnostics.content_selector.unwrap();
        assert!(!selector.matched);
        assert!(report.diagnostics.attempts[0].candidate_count > 0);
        assert!(!report.diagnostics.attempts[0].entry_points.is_empty());
    }

    #[test]
    fn metadata_expands_image_domain_favicon_and_deduplicated_author() {
        let article = extract(
            r#"
            <html><head>
                <title>Metadata Story</title>
                <link rel="canonical" href="https://www.example.com/story/one">
                <link rel="icon" href="/icon.png">
                <meta property="og:image" content="/lead.jpg">
                <meta name="author" content="Ada Lovelace, Ada Lovelace; Grace Hopper">
            </head><body>
                <article><p>This article has enough text and punctuation to be extracted as the main content.</p></article>
            </body></html>
            "#,
            Some("https://www.example.com/story/one"),
            &ReadabilityOptions { char_threshold: 0, ..Default::default() },
        )
        .unwrap()
        .unwrap();

        assert_eq!(article.byline.as_deref(), Some("Ada Lovelace, Grace Hopper"));
        assert_eq!(article.image.as_deref(), Some("https://www.example.com/lead.jpg"));
        assert_eq!(article.domain.as_deref(), Some("example.com"));
        assert_eq!(article.favicon.as_deref(), Some("https://www.example.com/icon.png"));
    }

    #[test]
    fn prefers_specific_heading_over_generic_site_title_and_cleans_header() {
        let article = extract(
            r#"
            <html><head>
                <title>Example Daily</title>
                <meta property="og:title" content="Example Daily">
                <meta property="og:site_name" content="Example Daily">
            </head><body>
                <article>
                    <header>
                        <h1>Specific Article Headline About Metadata</h1>
                        <div class="byline author-card">
                            <img src="/avatar.jpg" alt="">
                            By Ada Lovelace Published May 1, 2026
                        </div>
                        <time datetime="2026-05-01T12:00:00Z">May 1, 2026</time>
                        <figure class="hero"><img src="/hero.jpg" alt="Lead image"></figure>
                    </header>
                    <p>This article body is long enough, punctuated enough, and concrete enough to be selected without carrying the header chrome into the readable article.</p>
                    <p>The second paragraph should remain as normal body content after metadata and hero cleanup.</p>
                </article>
            </body></html>
            "#,
            Some("https://example.com/story"),
            &ReadabilityOptions { char_threshold: 0, ..Default::default() },
        )
        .unwrap()
        .unwrap();

        assert_eq!(
            article.title.as_deref(),
            Some("Specific Article Headline About Metadata")
        );
        assert_eq!(article.byline.as_deref(), Some("Ada Lovelace"));
        assert_eq!(article.published_time.as_deref(), Some("2026-05-01T12:00:00Z"));
        assert!(article.text_content.contains("This article body is long enough"));
        assert!(
            !article
                .text_content
                .contains("Specific Article Headline About Metadata")
        );
        assert!(!article.text_content.contains("Ada Lovelace"));
        assert!(!article.content.contains("hero.jpg"));
    }

    #[test]
    fn normalization_cleans_common_output_shapes_before_markdown() {
        let article = extract(
            r##"
            <html><head><title>Code Story</title></head><body>
                <article>
                    <h1>Code Story</h1>
                    <h2><a href="#code">Code Samples</a></h2>
                    <p>Alpha<wbr>Beta<br><br><br>Gamma</p>
                    <pre>let value = 1;</pre>
                    <div><span></span></div>
                </article>
            </body></html>
            "##,
            None,
            &ReadabilityOptions { char_threshold: 0, ..Default::default() },
        )
        .unwrap()
        .unwrap();

        assert!(!article.content.contains("<wbr"));
        assert!(!article.content.contains("<br><br><br>"));
        assert!(article.content.contains("<h2>Code Samples</h2>"));
        assert!(article.content.contains("<pre><code>let value = 1;</code></pre>"));
        assert!(article.markdown.contains("## Code Samples"));
        assert!(article.markdown.contains("    let value = 1;"));
    }

    #[test]
    fn removes_trailing_page_chrome_after_article_body() {
        let article = extract(
            r#"
            <html><head><title>Cleanup Story</title></head><body>
                <main>
                    <article>
                        <h1>Cleanup Story</h1>
                        <p>This article opens with enough detail, punctuation, and concrete prose to be selected as readable content.</p>
                        <p>This article continues with a second paragraph, so the following blocks are trailing page chrome.</p>
                        <section class="related-articles">
                            <h2>Related articles</h2>
                            <ul>
                                <li><a href="/a">One related link</a></li>
                                <li><a href="/b">Two related link</a></li>
                                <li><a href="/c">Three related link</a></li>
                            </ul>
                        </section>
                        <section id="comments">
                            <h2>Comments</h2>
                            <p>Join the discussion below.</p>
                        </section>
                    </article>
                </main>
            </body></html>
            "#,
            Some("https://example.com/cleanup"),
            &ReadabilityOptions { char_threshold: 0, ..Default::default() },
        )
        .unwrap()
        .unwrap();

        assert!(
            article
                .text_content
                .contains("following blocks are trailing page chrome")
        );
        assert!(!article.text_content.contains("Related articles"));
        assert!(!article.text_content.contains("Join the discussion"));
    }

    #[test]
    fn removes_reference_page_table_of_contents_without_url_profile() {
        let article = extract(
            r##"
            <html><head><title>Reference Entry</title></head><body>
                <main>
                    <article>
                        <h1>Reference Entry</h1>
                        <div id="toc">
                            <h2>Contents</h2>
                            <ol>
                                <li><a href="#history">History</a></li>
                                <li><a href="#software">Software</a></li>
                            </ol>
                        </div>
                        <p>This reference entry has enough real article text, punctuation, and detail to pass extraction.</p>
                        <h2 id="history">History</h2>
                        <p>The body explains the topic without keeping the navigation table of contents in reader output.</p>
                    </article>
                </main>
            </body></html>
            "##,
            None,
            &ReadabilityOptions { char_threshold: 0, ..Default::default() },
        )
        .unwrap()
        .unwrap();

        assert!(!article.text_content.contains("Contents"));
        assert!(!article.text_content.contains("Software"));
        assert!(article.text_content.contains("The body explains the topic"));
    }

    #[test]
    fn removes_continue_reading_article_chrome() {
        let article = extract(
            r##"
            <html><head><title>Continue Story</title></head><body>
                <article>
                    <h1>Continue Story</h1>
                    <p>This article has enough substance, punctuation, and detail to pass the extraction threshold.</p>
                    <p>The second paragraph keeps the article body clearly above the minimum length for this regression.</p>
                    <a href="#whats-next">Continue reading the main story</a>
                </article>
            </body></html>
            "##,
            None,
            &ReadabilityOptions { char_threshold: 0, ..Default::default() },
        )
        .unwrap()
        .unwrap();

        assert!(!article.text_content.contains("Continue reading"));
        assert!(article.text_content.contains("second paragraph"));
    }

    #[test]
    fn preserves_short_link_text_that_appears_in_byline_metadata() {
        let article = extract(
            r#"
            <html>
                <head>
                    <title>Announcing Rust 1.83.0 | Rust Blog</title>
                    <meta name="author" content="The Rust Release Team">
                </head>
                <body>
                    <article>
                        <p>This article has enough substance, punctuation, and detail to pass the extraction threshold.</p>
                        <p>
                            Check out everything that changed in
                            <a rel="external" href="https://github.com/rust-lang/rust/releases/tag/1.83.0">Rust</a>,
                            <a rel="external" href="https://doc.rust-lang.org/nightly/cargo/CHANGELOG.html">Cargo</a>,
                            and
                            <a rel="external" href="https://github.com/rust-lang/rust-clippy/blob/master/CHANGELOG.md">Clippy</a>.
                        </p>
                    </article>
                </body>
            </html>
            "#,
            None,
            &ReadabilityOptions { char_threshold: 0, ..Default::default() },
        )
        .unwrap()
        .unwrap();

        assert!(article.text_content.contains("changed in Rust, Cargo, and Clippy"));
        assert!(
            article
                .markdown
                .contains("[Rust](https://github.com/rust-lang/rust/releases/tag/1.83.0)")
        );
    }

    #[test]
    fn preserves_footnotes_while_removing_chrome_after_them() {
        let article = extract(
            r##"
            <html><head><title>Notes Story</title></head><body>
                <article>
                    <h1>Notes Story</h1>
                    <p>This article has enough substance, punctuation, and references to keep the body readable.<sup><a href="#fn1">1</a></sup></p>
                    <section class="footnotes">
                        <h2>Footnotes</h2>
                        <ol>
                            <li id="fn1">A legitimate note that should remain attached to the article.</li>
                        </ol>
                    </section>
                    <aside class="partner-offer">
                        <h2>Partner offers</h2>
                        <a href="/deal">Mortgage offer</a>
                        <a href="/card">Finance widget</a>
                        <a href="/jobs">Jobs widget</a>
                    </aside>
                </article>
            </body></html>
            "##,
            Some("https://example.com/notes"),
            &ReadabilityOptions { char_threshold: 0, ..Default::default() },
        )
        .unwrap()
        .unwrap();

        assert!(article.text_content.contains("legitimate note"));
        assert!(!article.text_content.contains("Partner offers"));
        assert!(!article.text_content.contains("Mortgage offer"));
    }

    #[test]
    fn removes_newsletter_category_and_next_article_tail_sections() {
        let article = extract(
            r#"
            <html><head><title>Article Tail</title></head><body>
                <div id="postContent">
                    <div id="postBody">
                        <section>
                            <p>This article body is long enough, punctuated enough, and concrete enough to be selected from a nested article page.</p>
                            <p>The final paragraph should remain as the end of the article.</p>
                        </section>
                        <section id="newsletter">
                            <div>The Daily Newsletter</div>
                            <p><em>Get highlights of the most important news delivered to your email inbox</em></p>
                        </section>
                        <section>
                            <div><h2>Also in <span>Physics</span></h2></div>
                        </section>
                        <div>
                            <section>
                                <div>
                                    <h2>Next article</h2>
                                    <div>Solution: A puzzle teaser</div>
                                </div>
                            </section>
                            <a href="/next"></a>
                        </div>
                    </div>
                </div>
            </body></html>
            "#,
            Some("https://example.com/story"),
            &ReadabilityOptions { char_threshold: 0, ..Default::default() },
        )
        .unwrap()
        .unwrap();

        assert!(article.text_content.contains("final paragraph should remain"));
        assert!(!article.text_content.contains("The Daily Newsletter"));
        assert!(!article.text_content.contains("Also in Physics"));
        assert!(!article.text_content.contains("Next article"));
    }

    #[test]
    fn recovery_hooks_expose_mobile_and_shadow_dom_content() {
        let mobile = extract_with_diagnostics(
            r#"
            <html><head>
                <style>@media (max-width: 600px) { .mobile-article { display: block; } }</style>
            </head><body>
                <article class="mobile-article" style="display: none">
                    <p>This mobile article has enough text, punctuation, and detail to become readable after display recovery.</p>
                </article>
            </body></html>
            "#,
            None,
            &ReadabilityOptions { char_threshold: 0, ..Default::default() },
        )
        .unwrap();
        let mobile_article = mobile.article.unwrap();
        assert!(mobile_article.text_content.contains("mobile article"));
        assert!(mobile.diagnostics.attempts[0].recovery.mobile_rules_applied > 0);

        let shadow = extract_with_diagnostics(
            r#"
            <html><body>
                <x-story>
                    <template shadowrootmode="open">
                        <article><p>This shadow article has enough text and punctuation to become readable after flattening.</p></article>
                    </template>
                </x-story>
            </body></html>
            "#,
            None,
            &ReadabilityOptions { char_threshold: 0, ..Default::default() },
        )
        .unwrap();
        let shadow_article = shadow.article.unwrap();
        assert!(shadow_article.text_content.contains("shadow article"));
        assert!(shadow.diagnostics.attempts[0].recovery.shadow_roots_flattened > 0);
    }

    #[test]
    fn prefers_focused_main_over_body_app_shell() {
        let report = extract_with_diagnostics(
            r#"
            <html><head><title>Focused docs</title></head><body class="antialiased">
                <nav>
                    <a href="/a">Overview</a><a href="/b">SDKs</a><a href="/c">API</a>
                    <a href="/d">Guides</a><a href="/e">Examples</a><a href="/f">Changelog</a>
                </nav>
                <main id="content-container">
                    <h1>Focused docs</h1>
                    <p>This documentation page has enough focused prose, commas, and detail to be selected as the article root.</p>
                    <p>The body also contains an app shell, but this main element is the useful content readers requested.</p>
                </main>
                <footer><a href="/privacy">Privacy</a><a href="/terms">Terms</a></footer>
            </body></html>
            "#,
            Some("https://example.com/docs/code"),
            &ReadabilityOptions { char_threshold: 0, ..Default::default() },
        )
        .unwrap();
        let article = report.article.unwrap();
        let selected = report.diagnostics.attempts[0]
            .selected_root
            .as_ref()
            .map(|node| node.selector.as_str());

        assert_eq!(selected, Some("main#content-container"));
        assert!(article.text_content.contains("useful content readers requested"));
        assert!(!article.text_content.contains("OverviewSDKsAPI"));
    }

    #[test]
    fn removes_doc_controls_without_dropping_code_panels() {
        let article = extract(
            r#"
            <html><head><title>Tabbed code docs</title></head><body>
                <main>
                    <h1>Tabbed code docs</h1>
                    <p>This page explains a code sample with enough prose to be readable and selected correctly.</p>
                    <button aria-label="Copy page"><svg></svg>Copy page</button>
                    <div role="tablist"><button role="tab">JavaScript</button><button role="tab">Python</button></div>
                    <section>
                        <pre data-language="js"><code>client.extract(url)</code></pre>
                    </section>
                    <p>The code panel should remain while toolbar controls and orphan labels are removed.</p>
                </main>
            </body></html>
            "#,
            Some("https://example.com/docs/create/code"),
            &ReadabilityOptions { char_threshold: 0, ..Default::default() },
        )
        .unwrap()
        .unwrap();

        assert!(
            article
                .content
                .contains("<pre data-language=\"js\"><code data-language=\"js\">client.extract(url)</code></pre>")
        );
        assert!(!article.text_content.contains("Copy page"));
        assert!(!article.text_content.contains("JavaScriptPython"));
    }

    #[test]
    fn matches_representative_fixture_metadata() {
        for name in [
            "wikipedia",
            "base-url-base-element",
            "article-author-tag",
            "parsely-metadata",
        ] {
            let fixture = lectito_fixtures::load_fixture(name).unwrap();
            let article = extract(
                &fixture.source,
                Some("http://fakehost/test/page.html"),
                &ReadabilityOptions { char_threshold: 0, ..Default::default() },
            )
            .unwrap()
            .unwrap();

            let expected = fixture.expected_metadata;
            if let Some(title) = expected.get("title").and_then(serde_json::Value::as_str) {
                assert_eq!(article.title.as_deref(), Some(title), "{name} title");
            }
            if let Some(byline) = expected.get("byline").and_then(serde_json::Value::as_str) {
                assert_eq!(article.byline.as_deref(), Some(byline), "{name} byline");
            }
            if let Some(excerpt) = expected.get("excerpt").and_then(serde_json::Value::as_str) {
                assert_eq!(
                    article.excerpt.as_deref().map(normalize_spaces),
                    Some(normalize_spaces(excerpt)),
                    "{name} excerpt"
                );
            }
            assert!(article.length > 0, "{name} should have text");
            assert!(
                article.content.contains("readability-page-1"),
                "{name} should be wrapped"
            );
        }
    }

    #[test]
    fn returns_content_for_representative_fixture_subset() {
        let names = [
            "wikipedia",
            "dropbox-blog",
            "cnet",
            "base-url-base-element",
            "keep-images",
            "replace-brs",
            "article-author-tag",
            "parsely-metadata",
        ];

        for name in names {
            let fixture = lectito_fixtures::load_fixture(name).unwrap();
            let article = extract(
                &fixture.source,
                Some("http://fakehost/test/page.html"),
                &ReadabilityOptions { char_threshold: 0, ..Default::default() },
            )
            .unwrap()
            .unwrap();

            assert!(article.length > 100, "{name} should have meaningful text");
            assert!(
                article.content.contains("readability-page-1"),
                "{name} should be wrapped"
            );
            assert!(
                !fixture.expected_content.trim().is_empty(),
                "{name} fixture should include expected content"
            );
        }
    }
}
