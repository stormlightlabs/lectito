use super::dom_tree::DomTree;
use super::metadata::text_similarity;
use super::parse::{Document, Element};
use super::postprocess::{PostProcessConfig, TableKind, classify_table_element, postprocess_html};
use super::preprocess::hidden_reason;
use super::scoring::{ScoreConfig, ScoreResult, calculate_score, link_density};
use super::siteconfig::{SiteConfig, SiteConfigProcessing, SiteConfigXPath};
use super::utils;
use super::{LectitoError, Result, preprocess};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::OnceLock;

/// Configuration for content extraction
#[derive(Debug, Clone)]
pub struct ExtractConfig {
    /// Minimum score threshold for top candidate
    pub min_score_threshold: f64,
    /// Maximum number of top candidates to track
    pub max_top_candidates: usize,
    /// Minimum character threshold for content
    pub char_threshold: usize,
    /// Maximum elements to consider
    pub max_elements: usize,
    /// Sibling score threshold (multiplier of top score)
    pub sibling_threshold: f64,
    /// Whether to remove selector-matched clutter before scoring
    pub pre_score_selector_removal: bool,
    /// Post-processing configuration
    pub postprocess: PostProcessConfig,
}

impl Default for ExtractConfig {
    fn default() -> Self {
        Self {
            min_score_threshold: 10.0,
            max_top_candidates: 5,
            char_threshold: 500,
            max_elements: 1000,
            sibling_threshold: 0.2,
            pre_score_selector_removal: true,
            postprocess: PostProcessConfig::default(),
        }
    }
}

/// A candidate element with its score
#[derive(Debug, Clone)]
pub struct Candidate<'a> {
    /// The element itself
    pub element: Element<'a>,
    /// The calculated score result
    pub score_result: ScoreResult,
}

/// The result of content extraction
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractionDiagnostics {
    /// Approximate word count of the full page before extraction.
    pub page_word_count: usize,
    /// Approximate text length of the full page before extraction.
    pub page_text_chars: usize,
    /// Word count of the extracted content.
    pub content_word_count: usize,
    /// Character count of the extracted content text.
    pub content_text_chars: usize,
    /// Extracted word count divided by page word count.
    pub content_word_ratio: f64,
    /// Top candidate score after candidate selection.
    pub top_candidate_score: Option<f64>,
    /// Score of the runner-up candidate when available.
    pub second_candidate_score: Option<f64>,
    /// Normalized spread between the best and second-best candidates.
    pub score_spread: Option<f64>,
    /// Link density of the selected candidate.
    pub top_candidate_link_density: Option<f64>,
    /// Candidate summaries for the strongest scoring nodes.
    pub candidate_scores: Vec<CandidateScoreDiagnostic>,
    /// Nodes removed before scoring because they matched clutter heuristics.
    pub removal_log: Vec<RemovalLogEntry>,
    /// Pass execution history, populated by the higher-level readability pipeline.
    pub pass_history: Vec<ExtractionPassDiagnostic>,
    /// Name of the pass that produced the final extraction.
    pub selected_pass: Option<String>,
    /// Matched site extractor, when extraction bypassed generic scoring.
    pub site_extractor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateScoreDiagnostic {
    pub tag_name: String,
    pub class: Option<String>,
    pub id: Option<String>,
    pub text_chars: usize,
    pub word_count: usize,
    pub base_score: f64,
    pub class_weight: f64,
    pub content_density: f64,
    pub link_density: f64,
    pub final_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemovalLogEntry {
    pub reason: String,
    pub selector: Option<String>,
    pub tag_name: String,
    pub attrs: Option<String>,
    pub text_chars: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionPassDiagnostic {
    pub name: String,
    pub strategy: String,
    pub succeeded: bool,
    pub word_count: usize,
    pub top_score: Option<f64>,
    pub confidence: Option<f64>,
    pub min_score_threshold: f64,
    pub remove_unlikely: bool,
    pub remove_hidden: bool,
    pub used_site_extractor: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExtractedContent {
    /// The main content element
    pub content: String,
    /// The top candidate score
    pub top_score: f64,
    /// Number of elements extracted
    pub element_count: usize,
    /// Confidence score for the extraction (0.0-1.0).
    pub confidence: f64,
    /// Diagnostics captured during extraction.
    pub diagnostics: ExtractionDiagnostics,
}

impl<'a> Candidate<'a> {
    /// Create a new candidate from an element
    fn new(element: Element<'a>, score_result: ScoreResult) -> Self {
        Self { element, score_result }
    }

    /// Get the final score of this candidate
    fn score(&self) -> f64 {
        self.score_result.final_score
    }
}

#[derive(Debug, Clone)]
struct EntryPointCandidate<'a> {
    candidate: Candidate<'a>,
    selector_index: usize,
    word_count: usize,
    content_score: f64,
}

impl<'a> EntryPointCandidate<'a> {
    fn score(&self) -> f64 {
        self.candidate.score()
    }
}

/// Tags that are considered potential content containers
const CANDIDATE_TAGS: &[&str] = &[
    "div",
    "article",
    "section",
    "main",
    "p",
    "pre",
    "blockquote",
    "td",
    "table",
];

/// Priority-ordered entry points for main-content detection.
const ENTRY_POINT_SELECTORS: &[&str] = &[
    "#post",
    ".post-content",
    ".article-body",
    ".entry-content",
    ".markdown-body",
    "#content",
    "#mw-content-text",
    ".mw-parser-output",
    "article",
    "[role='main']",
    "main",
    "body",
];
const MIN_ENTRY_POINT_WORDS: usize = 50;
const AGGREGATE_TOP_SCORE_RATIO: f64 = 0.75;
const AGGREGATE_MIN_CANDIDATES: usize = 3;
const SIBLING_SHARED_CLASS_BONUS_RATIO: f64 = 0.2;

/// Exact selectors that are safe to remove before scoring.
const PRE_SCORE_EXACT_SELECTORS: &[&str] = &[
    "nav",
    "aside",
    "footer",
    "[role='navigation']",
    "[role='complementary']",
    ".sidebar",
    "#sidebar",
    ".toc",
    "#toc",
    ".navbox",
    ".reflist",
    ".mw-references-wrap",
    ".mw-editsection",
    ".mw-editsection-bracket",
    ".hatnote",
    ".shortdescription",
    "sup.reference",
    "span.cite-bracket",
    "ol.references",
    "table.infobox",
    "form[action*='comment']",
    "form[action*='subscribe']",
    "[data-testid*='sidebar']",
    "[data-testid*='related']",
    "[data-component*='sidebar']",
    "[data-component*='related']",
];

fn pre_score_partial_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)(advert|banner|breadcrumb|comment|cookie|consent|editsection|footer|infobox|nav|navbox|newsletter|pager|pagination|popup|promo|reference|related|share|sidebar|social|sponsor|subscribe|toc|toolbar|widget)",
        )
        .unwrap()
    })
}

fn pre_score_positive_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)(article|body|content|entry|hentry|h-entry|main|page|post|text|blog|story)").unwrap()
    })
}

fn is_heading_tag(tag: &str) -> bool {
    matches!(tag, "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
}

fn selector_attrs(element: &Element<'_>) -> String {
    [
        element.attr("class").unwrap_or(""),
        element.attr("id").unwrap_or(""),
        element.attr("data-component").unwrap_or(""),
        element.attr("data-test").unwrap_or(""),
        element.attr("data-testid").unwrap_or(""),
        element.attr("data-test-id").unwrap_or(""),
        element.attr("data-qa").unwrap_or(""),
        element.attr("data-cy").unwrap_or(""),
    ]
    .join(" ")
    .to_lowercase()
}

fn should_remove_partial_selector_candidate(element: &Element<'_>) -> bool {
    let tag = element.tag_name();
    if matches!(
        tag.as_str(),
        "article"
            | "main"
            | "body"
            | "html"
            | "pre"
            | "code"
            | "math"
            | "figure"
            | "figcaption"
            | "iframe"
            | "embed"
            | "object"
            | "video"
            | "audio"
            | "source"
    ) || is_heading_tag(&tag)
    {
        return false;
    }
    if tag == "table" && classify_table_element(element) == TableKind::Data {
        return false;
    }

    let attrs = selector_attrs(element);
    if attrs.trim().is_empty() || !pre_score_partial_regex().is_match(&attrs) {
        return false;
    }
    if pre_score_positive_regex().is_match(&attrs) {
        return false;
    }
    if element
        .select("pre, code, math, figure, iframe, embed, object, video, audio, source")
        .ok()
        .is_some_and(|els| !els.is_empty())
    {
        return false;
    }

    let text_len = element.text().chars().count();
    let paragraph_count = element.select("p").map(|els| els.len()).unwrap_or(0);
    let ld = link_density(element);

    if text_len > 1800 || paragraph_count > 5 {
        return false;
    }
    if matches!(tag.as_str(), "div" | "section") && text_len > 600 && ld < 0.35 {
        return false;
    }

    match tag.as_str() {
        "div" | "section" | "aside" | "nav" | "ul" | "ol" | "li" | "form" | "table" => true,
        "p" | "span" => text_len <= 240,
        _ => text_len <= 160,
    }
}

fn remove_html_snippets(mut html: String, mut snippets: Vec<String>) -> String {
    snippets.sort_by_key(|b| std::cmp::Reverse(b.len()));
    for snippet in snippets {
        if snippet.is_empty() {
            continue;
        }
        if let Some(idx) = html.find(&snippet) {
            html.replace_range(idx..idx + snippet.len(), "");
        }
    }
    html
}

#[derive(Default)]
struct PrepareScoringResult {
    document: Option<Document>,
    removal_log: Vec<RemovalLogEntry>,
}

fn prepare_scoring_document(doc: &Document, config: &ExtractConfig) -> PrepareScoringResult {
    if !config.pre_score_selector_removal {
        return PrepareScoringResult::default();
    }

    let original_html = doc.as_string();
    let mut snippets = Vec::new();
    let mut removal_log = Vec::new();
    let protected_html = protected_cleanup_html(doc);

    for selector in PRE_SCORE_EXACT_SELECTORS {
        if let Ok(elements) = doc.select(selector) {
            for element in elements {
                if should_protect_from_cleanup(&element, &protected_html) {
                    continue;
                }
                snippets.push(element.outer_html());
                removal_log.push(RemovalLogEntry {
                    reason: "exact-selector".to_string(),
                    selector: Some((*selector).to_string()),
                    tag_name: element.tag_name(),
                    attrs: trimmed_selector_attrs(&element),
                    text_chars: element.text().chars().count(),
                });
            }
        }
    }

    if let Ok(elements) = doc.select("*") {
        for element in elements {
            if should_protect_from_cleanup(&element, &protected_html) {
                continue;
            }
            if should_remove_partial_selector_candidate(&element) {
                snippets.push(element.outer_html());
                removal_log.push(RemovalLogEntry {
                    reason: "partial-selector-pattern".to_string(),
                    selector: None,
                    tag_name: element.tag_name(),
                    attrs: trimmed_selector_attrs(&element),
                    text_chars: element.text().chars().count(),
                });
            }
        }
    }

    if snippets.is_empty() {
        return PrepareScoringResult { document: None, removal_log };
    }

    let filtered_html = remove_html_snippets(original_html.clone(), snippets);
    if filtered_html == original_html {
        return PrepareScoringResult { document: None, removal_log };
    }

    PrepareScoringResult {
        document: Document::parse_with_base_url(&filtered_html, doc.base_url().cloned()).ok(),
        removal_log,
    }
}

fn protected_cleanup_html(doc: &Document) -> HashSet<String> {
    doc.select("figure, figure *, iframe, embed, object, video, audio, source")
        .unwrap_or_default()
        .into_iter()
        .map(|element| element.outer_html())
        .collect()
}

fn should_protect_from_cleanup(element: &Element<'_>, protected_html: &HashSet<String>) -> bool {
    let tag = element.tag_name();
    if matches!(
        tag.as_str(),
        "figure" | "figcaption" | "iframe" | "embed" | "object" | "video" | "audio" | "source"
    ) {
        return true;
    }

    if protected_html.contains(&element.outer_html()) {
        return true;
    }

    element
        .select("figure, iframe, embed, object, video, audio, source")
        .ok()
        .is_some_and(|matches| !matches.is_empty())
}

fn trimmed_selector_attrs(element: &Element<'_>) -> Option<String> {
    let attrs = selector_attrs(element);
    let trimmed = attrs.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// Identify all candidate elements from the document
fn identify_candidates<'a>(
    doc: &'a Document, config: &ExtractConfig, score_config: &ScoreConfig,
) -> Vec<Candidate<'a>> {
    let mut candidates = Vec::new();
    let max_elements = if config.max_elements == 0 { usize::MAX } else { config.max_elements };
    let mut scanned = 0usize;

    for tag in CANDIDATE_TAGS {
        if let Ok(elements) = doc.select(tag) {
            for element in elements {
                if scanned >= max_elements {
                    return candidates;
                }
                scanned += 1;
                let tag_name = element.tag_name();
                if tag_name == "table" && classify_table_element(&element) == TableKind::Layout {
                    continue;
                }
                let text = element.text();
                if !matches!(tag_name.as_str(), "article" | "section" | "main")
                    && text.chars().count() < config.char_threshold / 10
                {
                    continue;
                }

                let score_result = calculate_score(&element, score_config);
                candidates.push(Candidate::new(element, score_result));
            }
        }
    }

    candidates
}

fn identify_entry_point_candidates<'a>(doc: &'a Document, score_config: &ScoreConfig) -> Vec<EntryPointCandidate<'a>> {
    let mut candidates = Vec::new();
    let selector_count = ENTRY_POINT_SELECTORS.len();

    for (index, selector) in ENTRY_POINT_SELECTORS.iter().enumerate() {
        let Ok(elements) = doc.select(selector) else {
            continue;
        };

        for element in elements {
            let words = utils::count_words(&element.text());
            if element.tag_name() != "body" && words < MIN_ENTRY_POINT_WORDS {
                continue;
            }

            let mut score_result = calculate_score(&element, score_config);
            let content_score = score_result.final_score;
            score_result.final_score += ((selector_count - index) * 40) as f64;

            candidates.push(EntryPointCandidate {
                candidate: Candidate::new(element, score_result),
                selector_index: index,
                word_count: words,
                content_score,
            });
        }
    }

    candidates
}

/// Propagate scores from candidates to their ancestors
///
/// This implements proper score propagation by traversing up the DOM tree:
/// - Parent elements get candidate_score / 2
/// - Grandparent elements get candidate_score / 3
///
/// This helps ensure that parent containers that contain high-scoring
/// content are also considered as candidates.
fn propagate_scores<'a>(candidates: &mut Vec<Candidate<'a>>, doc: &'a Document, dom_tree: &DomTree) {
    let score_config = ScoreConfig::default();
    let mut processed_elements: HashSet<String> = HashSet::new();
    let mut additional_candidates = Vec::new();

    for candidate in candidates.iter() {
        let cand_html = candidate.element.outer_html();
        processed_elements.insert(candidate_lookup_key(&candidate.element.tag_name(), &cand_html));
    }

    for candidate in candidates.iter() {
        let candidate_score = candidate.score();
        let candidate_html = candidate.element.outer_html();
        let candidate_tag = candidate.element.tag_name();

        if let Some(parent_node) = dom_tree.get_parent_by_html(&candidate_html, &candidate_tag) {
            promote_ancestor_candidate(
                doc,
                &mut processed_elements,
                &mut additional_candidates,
                (&parent_node.tag_name, &parent_node.html),
                candidate_score,
                2.0,
                &score_config,
            );

            if let Some(parent_id) = parent_node.parent_id
                && let Some(grandparent_node) = dom_tree.get_parent(parent_id)
            {
                promote_ancestor_candidate(
                    doc,
                    &mut processed_elements,
                    &mut additional_candidates,
                    (&grandparent_node.tag_name, &grandparent_node.html),
                    candidate_score,
                    3.0,
                    &score_config,
                );
            }
        }
    }

    candidates.extend(additional_candidates);
}

fn candidate_lookup_key(tag_name: &str, html: &str) -> String {
    if html.len() > 200 {
        let truncated = truncate_at_char_boundary(html, 200);
        format!("{tag_name}-{truncated}")
    } else {
        format!("{tag_name}-{html}")
    }
}

fn promote_ancestor_candidate<'a>(
    doc: &'a Document, processed_elements: &mut HashSet<String>, additional_candidates: &mut Vec<Candidate<'a>>,
    ancestor: (&str, &str), candidate_score: f64, boost_divisor: f64, score_config: &ScoreConfig,
) {
    let (tag_name, html) = ancestor;
    let key = candidate_lookup_key(tag_name, html);
    if processed_elements.contains(&key) {
        return;
    }

    let Some(element) = element_for_node(doc, tag_name, html) else {
        return;
    };

    let mut boosted_result = calculate_score(&element, score_config);
    boosted_result.final_score += candidate_score / boost_divisor;
    additional_candidates.push(Candidate::new(element, boosted_result));
    processed_elements.insert(key);
}

/// Select the top candidate from the list.
///
/// Returns an owned candidate so later pipeline stages can move the candidate list
/// without holding references into it.
fn select_top_candidate<'a>(candidates: &[Candidate<'a>], config: &ExtractConfig) -> Result<Candidate<'a>> {
    if candidates.is_empty() {
        return Err(LectitoError::NoContent);
    }

    let top_candidate = candidates
        .iter()
        .max_by(|a, b| compare_candidates(a, b).unwrap_or(std::cmp::Ordering::Equal))
        .map(copy_candidate)
        .unwrap();

    if top_candidate.score() < config.min_score_threshold {
        return Err(LectitoError::NotReadable { score: top_candidate.score(), threshold: config.min_score_threshold });
    }

    Ok(top_candidate)
}

fn node_id_for_element(element: &Element<'_>, dom_tree: &DomTree) -> Option<usize> {
    let html = element.outer_html();
    let tag = element.tag_name();
    dom_tree
        .find_by_html(&html, &tag)
        .and_then(|node| (node.html == html).then_some(()))
        .and_then(|_| {
            (0..dom_tree.len()).find(|idx| {
                dom_tree
                    .get_node(*idx)
                    .is_some_and(|node| node.tag_name == tag && node.html == html)
            })
        })
}

fn node_depth(node_id: usize, dom_tree: &DomTree) -> usize {
    let mut depth = 0usize;
    let mut current = Some(node_id);

    while let Some(id) = current {
        current = dom_tree.get_node(id).and_then(|node| node.parent_id);
        if current.is_some() {
            depth += 1;
        }
    }

    depth
}

fn is_descendant_of(candidate_id: usize, ancestor_id: usize, dom_tree: &DomTree) -> bool {
    let mut current = Some(candidate_id);

    while let Some(id) = current {
        if id == ancestor_id {
            return true;
        }
        current = dom_tree.get_node(id).and_then(|node| node.parent_id);
    }

    false
}

fn element_for_node<'a>(doc: &'a Document, tag_name: &str, html: &str) -> Option<Element<'a>> {
    doc.select(tag_name)
        .ok()?
        .into_iter()
        .find(|element| element.outer_html() == html)
}

fn nearest_cluster_ancestor(top_id: usize, cluster_ids: &[usize], dom_tree: &DomTree) -> Option<usize> {
    let mut current = dom_tree.get_node(top_id)?.parent_id;

    while let Some(node_id) = current {
        let node = dom_tree.get_node(node_id)?;
        let clustered = cluster_ids
            .iter()
            .filter(|candidate_id| is_descendant_of(**candidate_id, node_id, dom_tree))
            .count();

        if clustered >= AGGREGATE_MIN_CANDIDATES && !matches!(node.tag_name.as_str(), "body" | "html") {
            return Some(node_id);
        }

        current = node.parent_id;
    }

    None
}

fn aggregate_clustered_candidates<'a>(
    doc: &'a Document, candidates: &[Candidate<'a>], dom_tree: &DomTree, score_config: &ScoreConfig,
) -> Option<Candidate<'a>> {
    let top_candidate = candidates.first()?;
    let top_id = node_id_for_element(&top_candidate.element, dom_tree)?;
    let top_threshold = top_candidate.score() * AGGREGATE_TOP_SCORE_RATIO;

    let near_top = candidates
        .iter()
        .filter(|candidate| candidate.score() >= top_threshold)
        .filter_map(|candidate| node_id_for_element(&candidate.element, dom_tree).map(|id| (candidate, id)))
        .collect::<Vec<_>>();

    if near_top.len() < AGGREGATE_MIN_CANDIDATES {
        return None;
    }

    let cluster_ids = near_top.iter().map(|(_, id)| *id).collect::<Vec<_>>();
    let ancestor_id = nearest_cluster_ancestor(top_id, &cluster_ids, dom_tree)?;
    let ancestor_node = dom_tree.get_node(ancestor_id)?;
    let ancestor = element_for_node(doc, &ancestor_node.tag_name, &ancestor_node.html)?;

    if ancestor.outer_html() == top_candidate.element.outer_html() {
        return None;
    }

    let top_words = utils::count_words(&top_candidate.element.text()).max(1);
    let ancestor_words = utils::count_words(&ancestor.text());
    if ancestor_words * 5 < top_words * 6 {
        return None;
    }

    let link_density = link_density(&ancestor);
    if link_density > 0.45 {
        return None;
    }

    let mut score_result = calculate_score(&ancestor, score_config);
    let bonus = near_top
        .iter()
        .map(|(candidate, _)| candidate.score() * 0.35)
        .sum::<f64>();
    if bonus <= 0.0 {
        return None;
    }

    score_result.final_score += bonus;
    Some(Candidate::new(ancestor, score_result))
}

fn class_tokens(element: &Element<'_>) -> HashSet<String> {
    element
        .attr("class")
        .unwrap_or("")
        .split_whitespace()
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .collect()
}

fn sibling_class_bonus(candidate: &Candidate<'_>, top_candidate_classes: &HashSet<String>, top_score: f64) -> f64 {
    if top_candidate_classes.is_empty() {
        return 0.0;
    }

    let candidate_classes = class_tokens(&candidate.element);
    if candidate_classes
        .iter()
        .any(|class_name| top_candidate_classes.contains(class_name))
    {
        top_score * SIBLING_SHARED_CLASS_BONUS_RATIO
    } else {
        0.0
    }
}

#[derive(Clone)]
struct SelectedBlock<'a> {
    element: Element<'a>,
    order: usize,
}

struct CandidateSelection<'a> {
    candidates: Vec<Candidate<'a>>,
    top_candidate: Candidate<'a>,
    dom_tree: Option<DomTree>,
}

fn copy_candidate<'a>(candidate: &Candidate<'a>) -> Candidate<'a> {
    Candidate::new(candidate.element.clone(), candidate.score_result.clone())
}

fn block_word_count(element: &Element<'_>) -> usize {
    let normalized = utils::normalize_whitespace(&element.text());
    utils::count_words(&normalized)
}

fn block_preference_score(element: &Element<'_>) -> usize {
    let mut score = block_word_count(element);
    let tag = element.tag_name();

    if matches!(tag.as_str(), "article" | "main" | "section") {
        score += 24;
    }
    if element
        .attr("role")
        .is_some_and(|role| role.eq_ignore_ascii_case("main"))
    {
        score += 20;
    }
    if element.select("h1, h2").ok().is_some_and(|els| !els.is_empty()) {
        score += 12;
    }
    if matches!(tag.as_str(), "header" | "h1" | "h2") {
        score += 6;
    }

    score
}

fn should_drop_overlapping_block(current: &SelectedBlock<'_>, other: &SelectedBlock<'_>) -> bool {
    let current_text = utils::normalize_whitespace(&current.element.text());
    let other_text = utils::normalize_whitespace(&other.element.text());

    if current_text.is_empty() || other_text.is_empty() || current_text == other_text && current.order == other.order {
        return false;
    }

    let current_pref = block_preference_score(&current.element);
    let other_pref = block_preference_score(&other.element);

    if current_text == other_text {
        return other_pref > current_pref || (other_pref == current_pref && other.order < current.order);
    }

    let current_words = utils::count_words(&current_text);
    let other_words = utils::count_words(&other_text);
    if current_words == 0 || other_words == 0 || other_words < current_words {
        return false;
    }

    let strongly_overlaps = other_text.contains(&current_text) || text_similarity(&current_text, &other_text) > 0.98;
    if !strongly_overlaps {
        return false;
    }

    other_words > current_words || other_pref > current_pref
}

fn prune_overlapping_blocks<'a>(blocks: Vec<SelectedBlock<'a>>) -> Vec<SelectedBlock<'a>> {
    let mut drop_indices = HashSet::new();

    for (idx, current) in blocks.iter().enumerate() {
        for (other_idx, other) in blocks.iter().enumerate() {
            if idx == other_idx {
                continue;
            }
            if should_drop_overlapping_block(current, other) {
                drop_indices.insert(idx);
                break;
            }
        }
    }

    blocks
        .into_iter()
        .enumerate()
        .filter(|(idx, _)| !drop_indices.contains(idx))
        .map(|(_, block)| block)
        .collect()
}

fn find_selected_candidate<'a>(
    blocks: &[SelectedBlock<'a>], candidates: &'a [Candidate<'a>],
) -> Option<&'a Candidate<'a>> {
    blocks
        .iter()
        .filter_map(|block| {
            candidates
                .iter()
                .find(|candidate| candidate.element.outer_html() == block.element.outer_html())
        })
        .max_by(|left, right| compare_candidates(left, right).unwrap_or(std::cmp::Ordering::Equal))
}

fn candidate_diagnostics(candidates: &[Candidate<'_>], limit: usize) -> Vec<CandidateScoreDiagnostic> {
    candidates
        .iter()
        .take(limit)
        .map(|candidate| CandidateScoreDiagnostic {
            tag_name: candidate.score_result.tag_name.clone(),
            class: candidate.score_result.class.clone(),
            id: candidate.score_result.id.clone(),
            text_chars: candidate.element.text().chars().count(),
            word_count: utils::count_words(&candidate.element.text()),
            base_score: candidate.score_result.base_score,
            class_weight: candidate.score_result.class_weight,
            content_density: candidate.score_result.content_density,
            link_density: candidate.score_result.link_density,
            final_score: candidate.score_result.final_score,
        })
        .collect()
}

fn normalized_score_spread(top_score: f64, second_score: Option<f64>) -> Option<f64> {
    second_score.map(|second| ((top_score - second) / top_score.abs().max(1.0)).clamp(0.0, 1.0))
}

fn clamp01(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

fn confidence_from_diagnostics(diagnostics: &ExtractionDiagnostics) -> f64 {
    let length_factor = clamp01(diagnostics.content_word_count as f64 / 900.0);
    let ratio_factor = clamp01(diagnostics.content_word_ratio / 0.35);
    let link_factor = diagnostics
        .top_candidate_link_density
        .map(|value| clamp01(1.0 - (value / 0.7)))
        .unwrap_or(0.5);
    let spread_factor = diagnostics.score_spread.unwrap_or(0.5);
    let retry_factor = diagnostics
        .selected_pass
        .as_deref()
        .map(pass_reliability_factor)
        .unwrap_or(0.8);
    let extractor_factor = if diagnostics.site_extractor.is_some() { 1.0 } else { 0.0 };

    clamp01(
        (length_factor * 0.28)
            + (link_factor * 0.22)
            + (spread_factor * 0.18)
            + (retry_factor * 0.16)
            + (ratio_factor * 0.08)
            + (extractor_factor * 0.08),
    )
}

pub(crate) fn pass_reliability_factor(pass_name: &str) -> f64 {
    match pass_name {
        "site-extractor" => 0.98,
        "pass-0-default" => 1.0,
        "pass-1-relaxed-selectors" => 0.88,
        "pass-2-hidden-disabled" => 0.76,
        "pass-3-hidden-subtree" => 0.62,
        "pass-4-no-score-threshold" => 0.52,
        "pass-5-schema-org" => 0.45,
        _ => 0.75,
    }
}

fn build_extraction_diagnostics(
    source_doc: &Document, content: &str, top_candidate: &Candidate<'_>, candidates: &[Candidate<'_>],
    removal_log: Vec<RemovalLogEntry>,
) -> ExtractionDiagnostics {
    let page_text = source_doc.text_content();
    let page_word_count = utils::count_words(&page_text);
    let content_text = Document::parse(content)
        .map(|doc| doc.text_content())
        .unwrap_or_default();
    let content_word_count = utils::count_words(&content_text);
    let top_score = top_candidate.score();
    let second_score = candidates
        .iter()
        .filter(|candidate| candidate.element.outer_html() != top_candidate.element.outer_html())
        .map(Candidate::score)
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    ExtractionDiagnostics {
        page_word_count,
        page_text_chars: page_text.chars().count(),
        content_word_count,
        content_text_chars: content_text.chars().count(),
        content_word_ratio: if page_word_count == 0 { 0.0 } else { content_word_count as f64 / page_word_count as f64 },
        top_candidate_score: Some(top_score),
        second_candidate_score: second_score,
        score_spread: normalized_score_spread(top_score, second_score),
        top_candidate_link_density: Some(top_candidate.score_result.link_density),
        candidate_scores: candidate_diagnostics(candidates, 5),
        removal_log,
        pass_history: Vec::new(),
        selected_pass: None,
        site_extractor: None,
    }
}

fn select_preferred_entry_candidate<'a>(
    entry_candidates: &[EntryPointCandidate<'a>], dom_tree: Option<&DomTree>,
) -> Option<EntryPointCandidate<'a>> {
    if entry_candidates.is_empty() {
        return None;
    }

    let mut ordered = entry_candidates.iter().collect::<Vec<_>>();
    ordered.sort_by(|a, b| {
        b.score()
            .partial_cmp(&a.score())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.selector_index.cmp(&b.selector_index))
            .then_with(|| b.word_count.cmp(&a.word_count))
    });

    let top = ordered[0];
    let Some(dom_tree) = dom_tree else {
        return Some(copy_entry_candidate(top));
    };
    let Some(top_id) = node_id_for_element(&top.candidate.element, dom_tree) else {
        return Some(copy_entry_candidate(top));
    };

    let preferred = ordered
        .into_iter()
        .filter(|candidate| {
            candidate.selector_index < top.selector_index && candidate.word_count > MIN_ENTRY_POINT_WORDS
        })
        .filter_map(|candidate| {
            let candidate_id = node_id_for_element(&candidate.candidate.element, dom_tree)?;
            if !is_descendant_of(candidate_id, top_id, dom_tree) || candidate_id == top_id {
                return None;
            }

            let siblings_at_priority = entry_candidates
                .iter()
                .filter(|other| other.selector_index == candidate.selector_index)
                .filter_map(|other| node_id_for_element(&other.candidate.element, dom_tree))
                .filter(|other_id| is_descendant_of(*other_id, top_id, dom_tree) && *other_id != top_id)
                .count();

            if siblings_at_priority > 1 {
                return None;
            }

            Some((candidate, node_depth(candidate_id, dom_tree)))
        })
        .max_by(|(a_candidate, a_depth), (b_candidate, b_depth)| {
            b_candidate
                .selector_index
                .cmp(&a_candidate.selector_index)
                .then_with(|| a_depth.cmp(b_depth))
                .then_with(|| {
                    a_candidate
                        .score()
                        .partial_cmp(&b_candidate.score())
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });

    preferred
        .map(|(candidate, _)| copy_entry_candidate(candidate))
        .or_else(|| Some(copy_entry_candidate(top)))
}

fn copy_entry_candidate<'a>(candidate: &EntryPointCandidate<'a>) -> EntryPointCandidate<'a> {
    EntryPointCandidate {
        candidate: copy_candidate(&candidate.candidate),
        selector_index: candidate.selector_index,
        word_count: candidate.word_count,
        content_score: candidate.content_score,
    }
}

fn should_prefer_scored_descendant(
    entry_candidate: &EntryPointCandidate<'_>, scored_candidate: &Candidate<'_>, dom_tree: Option<&DomTree>,
) -> bool {
    if !matches!(entry_candidate.candidate.element.tag_name().as_str(), "main" | "body") {
        return false;
    }

    let Some(dom_tree) = dom_tree else {
        return false;
    };
    let Some(entry_id) = node_id_for_element(&entry_candidate.candidate.element, dom_tree) else {
        return false;
    };
    let Some(scored_id) = node_id_for_element(&scored_candidate.element, dom_tree) else {
        return false;
    };
    if scored_id == entry_id || !is_descendant_of(scored_id, entry_id, dom_tree) {
        return false;
    }

    let entry_words = entry_candidate.word_count.max(1);
    let scored_words = utils::count_words(&scored_candidate.element.text());
    scored_words >= MIN_ENTRY_POINT_WORDS && scored_words * 10 >= entry_words * 6
}

fn entry_candidate_meets_threshold(entry_candidate: &EntryPointCandidate<'_>, config: &ExtractConfig) -> bool {
    let tag = entry_candidate.candidate.element.tag_name();
    tag != "body" || entry_candidate.content_score >= config.min_score_threshold
}

fn select_table_layout_candidate<'a>(doc: &'a Document, score_config: &ScoreConfig) -> Option<Candidate<'a>> {
    let tables = doc.select("table").ok()?;
    if !tables
        .iter()
        .any(|table| classify_table_element(table) == TableKind::Layout)
    {
        return None;
    }

    let mut candidates = Vec::new();

    for selector in ["td", "table"] {
        let Ok(elements) = doc.select(selector) else {
            continue;
        };
        for element in elements {
            let words = utils::count_words(&element.text());
            if words < MIN_ENTRY_POINT_WORDS {
                continue;
            }

            let link_density = link_density(&element);
            if link_density > 0.45 {
                continue;
            }

            let mut score_result = calculate_score(&element, score_config);
            score_result.final_score += if selector == "td" { 30.0 } else { 20.0 };
            candidates.push(Candidate::new(element, score_result));
        }
    }

    candidates
        .into_iter()
        .max_by(|a, b| compare_candidates(a, b).unwrap_or(std::cmp::Ordering::Equal))
}

/// Select siblings that should be included with the top candidate
///
/// Siblings are included if:
/// - They share the same parent
/// - Their score is >= top_score * sibling_threshold
/// - For P tags: link_density < 0.25 and text_length > 80 chars
fn select_siblings<'a>(
    doc: &'a Document, top_candidate: &Candidate<'a>, candidates: &[Candidate<'a>], config: &ExtractConfig,
    dom_tree: Option<&DomTree>,
) -> Vec<Element<'a>> {
    let mut siblings = Vec::new();
    let top_score = top_candidate.score();
    let sibling_threshold = top_score * config.sibling_threshold;
    let top_candidate_classes = class_tokens(&top_candidate.element);
    let dom_tree = match dom_tree {
        Some(tree) => tree,
        None => return siblings,
    };
    let top_parent_id = parent_id_for(&top_candidate.element, dom_tree);
    if top_parent_id.is_none() {
        return siblings;
    }

    for candidate in candidates {
        let adjusted_score = candidate.score() + sibling_class_bonus(candidate, &top_candidate_classes, top_score);
        if adjusted_score < sibling_threshold {
            continue;
        }

        let top_html = top_candidate.element.outer_html();
        let candidate_html = candidate.element.outer_html();

        if top_html != candidate_html {
            if parent_id_for(&candidate.element, dom_tree) != top_parent_id {
                continue;
            }
            if candidate.element.tag_name() == "p" {
                let text = candidate.element.text();
                let text_len = text.chars().count();

                if text_len > 80 {
                    let link_density = link_density(&candidate.element);
                    if link_density < 0.25 {
                        siblings.push(candidate.element.clone());
                    }
                }
            } else {
                siblings.push(candidate.element.clone());
            }
        }
    }

    let mut added_header = false;
    if let Ok(headers) = doc.select("header") {
        for header in headers {
            if parent_id_for(&header, dom_tree) != top_parent_id {
                let header_parent_id = parent_id_for(&header, dom_tree);
                if !shares_container(header_parent_id, top_parent_id, dom_tree) {
                    continue;
                }
            }
            if header.outer_html() == top_candidate.element.outer_html() {
                continue;
            }
            if header.select("h1").ok().and_then(|els| els.first().cloned()).is_none() {
                continue;
            }
            let text_len = header.text().trim().chars().count();
            if text_len < 10 {
                continue;
            }
            let link_density = link_density(&header);
            if link_density > 0.3 {
                continue;
            }
            if !siblings.iter().any(|s| s.outer_html() == header.outer_html()) {
                siblings.insert(0, header);
                added_header = true;
            }
        }
    }

    if !added_header
        && top_candidate.element.select("h1").ok().is_none_or(|els| els.is_empty())
        && let Ok(headings) = doc.select("h1")
        && let Some(heading) = headings.first()
    {
        let heading_text = heading.text().trim().to_string();
        if heading_text.len() > 5 {
            let link_density = link_density(heading);
            let top_text = top_candidate.element.text();
            if link_density <= 0.3
                && !top_text.contains(&heading_text)
                && !siblings.iter().any(|s| s.outer_html() == heading.outer_html())
                && top_candidate.element.outer_html() != heading.outer_html()
            {
                siblings.insert(0, heading.clone());
            }
        }
    }

    siblings
}

fn collect_candidates_from_doc<'a>(
    doc: &'a Document, config: &ExtractConfig, score_config: &ScoreConfig,
) -> Result<CandidateSelection<'a>> {
    let entry_candidates = identify_entry_point_candidates(doc, score_config);
    let mut candidates = identify_candidates(doc, config, score_config);
    if candidates.is_empty() {
        return Err(LectitoError::NoContent);
    }

    let dom_tree = crate::build_dom_tree(&doc.as_string()).ok();
    if let Some(tree) = dom_tree.as_ref() {
        propagate_scores(&mut candidates, doc, tree);
    }

    candidates.sort_by(|a, b| b.score().partial_cmp(&a.score()).unwrap_or(std::cmp::Ordering::Equal));
    let aggregated_candidate = dom_tree
        .as_ref()
        .and_then(|tree| aggregate_clustered_candidates(doc, &candidates, tree, score_config));

    candidates.truncate(config.max_top_candidates);
    let table_layout_candidate = select_table_layout_candidate(doc, score_config);
    let scored_top_candidate = match (select_top_candidate(&candidates, config).ok(), aggregated_candidate) {
        (Some(scored), Some(aggregated)) => Some(
            if compare_candidates(&aggregated, &scored).is_some_and(|order| order.is_gt()) {
                aggregated
            } else {
                scored
            },
        ),
        (Some(scored), None) => Some(scored),
        (None, Some(aggregated)) => Some(aggregated),
        (None, None) => None,
    };
    let preferred_entry_candidate = select_preferred_entry_candidate(&entry_candidates, dom_tree.as_ref())
        .filter(|candidate| candidate.score() >= config.min_score_threshold)
        .filter(|candidate| entry_candidate_meets_threshold(candidate, config));

    let top_candidate = match (preferred_entry_candidate, scored_top_candidate, table_layout_candidate) {
        (Some(entry_candidate), Some(scored_candidate), _)
            if should_prefer_scored_descendant(&entry_candidate, &scored_candidate, dom_tree.as_ref()) =>
        {
            scored_candidate
        }
        (Some(entry_candidate), _, _) => entry_candidate.candidate,
        (None, Some(scored_candidate), _) => scored_candidate,
        (None, None, Some(table_candidate)) if table_candidate.score() >= config.min_score_threshold => table_candidate,
        _ => select_top_candidate(&candidates, config)?,
    };

    Ok(CandidateSelection { candidates, top_candidate, dom_tree })
}

fn assemble_selected_blocks<'a>(
    doc: &'a Document, selection: &CandidateSelection<'a>, config: &ExtractConfig,
) -> Vec<SelectedBlock<'a>> {
    let siblings = select_siblings(
        doc,
        &selection.top_candidate,
        &selection.candidates,
        config,
        selection.dom_tree.as_ref(),
    );
    let (heading_blocks, body_blocks): (Vec<_>, Vec<_>) = siblings
        .into_iter()
        .partition(|element| matches!(element.tag_name().as_str(), "header" | "h1"));

    let mut selected_blocks = Vec::new();
    let mut order = 0usize;

    for element in heading_blocks {
        selected_blocks.push(SelectedBlock { element, order });
        order += 1;
    }

    selected_blocks.push(SelectedBlock { element: selection.top_candidate.element.clone(), order });
    order += 1;

    for element in body_blocks {
        selected_blocks.push(SelectedBlock { element, order });
        order += 1;
    }

    prune_overlapping_blocks(selected_blocks)
}

fn render_selected_blocks(doc: &Document, selected_blocks: &[SelectedBlock<'_>]) -> String {
    let mut content = selected_blocks
        .iter()
        .map(|block| block.element.outer_html())
        .collect::<Vec<_>>()
        .join("\n");

    if let Some(title) = doc.title() {
        content = ensure_title_heading(content, &title);
    }

    content
}

fn ensure_title_heading(content: String, title: &str) -> String {
    let has_h1 = content.contains("<h1");
    let content_text = Document::parse(&content)
        .map(|doc| doc.text_content())
        .unwrap_or_default();
    if has_h1 || content_text.contains(title) {
        return content;
    }

    let safe_title = utils::escape_html(title);
    format!("<h1>{safe_title}</h1>\n{content}")
}

fn finalize_extraction_result(
    source_doc: &Document, config: &ExtractConfig, selection: CandidateSelection<'_>,
    selected_blocks: Vec<SelectedBlock<'_>>, removal_log: Vec<RemovalLogEntry>,
) -> ExtractedContent {
    let content = postprocess_html(
        &render_selected_blocks(source_doc, &selected_blocks),
        &config.postprocess,
    );
    let primary_candidate =
        find_selected_candidate(&selected_blocks, &selection.candidates).unwrap_or(&selection.top_candidate);
    let element_count = selected_blocks.len();
    let diagnostics = build_extraction_diagnostics(
        source_doc,
        &content,
        primary_candidate,
        &selection.candidates,
        removal_log,
    );
    let confidence = confidence_from_diagnostics(&diagnostics);

    ExtractedContent { content, top_score: selection.top_candidate.score(), element_count, confidence, diagnostics }
}

/// Extract the main content from a document
///
/// This is the main entry point for content extraction. It:
/// 1. Identifies candidate elements
/// 2. Propagates scores to ancestors
/// 3. Selects the top candidate
/// 4. Includes relevant siblings
/// 5. Post-processes the extracted content
/// 6. Returns the cleaned content
pub fn extract_content(doc: &Document, config: &ExtractConfig) -> Result<ExtractedContent> {
    let prepared = prepare_scoring_document(doc, config);
    let score_config = ScoreConfig::default();
    let mut working_doc = prepared.document.as_ref().unwrap_or(doc);
    let selection = match collect_candidates_from_doc(working_doc, config, &score_config) {
        Ok(selection) => selection,
        Err(LectitoError::NoContent) if prepared.document.is_some() => {
            working_doc = doc;
            collect_candidates_from_doc(working_doc, config, &score_config)?
        }
        Err(error) => return Err(error),
    };
    let selected_blocks = assemble_selected_blocks(working_doc, &selection, config);

    Ok(finalize_extraction_result(
        doc,
        config,
        selection,
        selected_blocks,
        prepared.removal_log,
    ))
}

fn parent_id_for(element: &Element<'_>, dom_tree: &DomTree) -> Option<usize> {
    let html = element.outer_html();
    let tag = element.tag_name();
    dom_tree.find_by_html(&html, &tag).and_then(|node| node.parent_id)
}

fn shares_container(header_parent_id: Option<usize>, top_parent_id: Option<usize>, dom_tree: &DomTree) -> bool {
    if header_parent_id == top_parent_id {
        return true;
    }

    let top_grandparent_id = top_parent_id
        .and_then(|id| dom_tree.get_parent(id))
        .and_then(|node| node.parent_id);
    if header_parent_id == top_grandparent_id {
        return true;
    }

    if let Some(header_parent_id) = header_parent_id
        && let Some(parent_node) = dom_tree.get_parent(header_parent_id)
    {
        return parent_node.parent_id == top_parent_id;
    }

    false
}

fn truncate_at_char_boundary(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        return s;
    }

    let safe_len = s.floor_char_boundary(max_len);
    &s[..safe_len]
}

fn compare_candidates<'a>(a: &Candidate<'a>, b: &Candidate<'a>) -> Option<std::cmp::Ordering> {
    let score_order = a.score().partial_cmp(&b.score())?;
    if score_order != std::cmp::Ordering::Equal {
        return Some(score_order);
    }

    let a_tag = a.element.tag_name();
    let b_tag = b.element.tag_name();
    let tag_order = candidate_priority(&a_tag).cmp(&candidate_priority(&b_tag));
    if tag_order != std::cmp::Ordering::Equal {
        return Some(tag_order);
    }

    let a_len = a.element.text().chars().count();
    let b_len = b.element.text().chars().count();
    Some(a_len.cmp(&b_len))
}

fn candidate_priority(tag_name: &str) -> u8 {
    match tag_name {
        "article" | "main" | "section" => 3,
        "div" => 2,
        _ => 1,
    }
}

fn build_single_block_extraction(
    doc: &Document, content: String, content_text: Option<String>, top_score: f64, site_extractor: Option<&str>,
) -> ExtractedContent {
    let content_text = content_text.unwrap_or_else(|| {
        Document::parse(&content)
            .map(|doc| doc.text_content())
            .unwrap_or_default()
    });
    let page_text = doc.text_content();
    let page_word_count = utils::count_words(&page_text);
    let content_word_count = utils::count_words(&content_text);
    let diagnostics = ExtractionDiagnostics {
        page_word_count,
        page_text_chars: page_text.chars().count(),
        content_word_count,
        content_text_chars: content_text.chars().count(),
        content_word_ratio: content_word_count as f64 / page_word_count.max(1) as f64,
        top_candidate_score: Some(top_score),
        second_candidate_score: None,
        score_spread: None,
        top_candidate_link_density: None,
        candidate_scores: Vec::new(),
        removal_log: Vec::new(),
        pass_history: Vec::new(),
        selected_pass: None,
        site_extractor: site_extractor.map(|value| value.to_string()),
    };
    let confidence = confidence_from_diagnostics(&diagnostics);

    ExtractedContent { content, top_score, element_count: 1, confidence, diagnostics }
}

/// Extract content from the largest hidden element subtree (Pass 3 of multi-pass retry).
///
/// Scans for elements hidden via inline styles or CSS utility classes and returns
/// the subtree with the most text content. This targets pages that hide their main
/// content until JavaScript runs.
pub(crate) fn extract_largest_hidden_subtree(doc: &Document) -> Option<ExtractedContent> {
    let mut best_html = String::new();
    let mut best_word_count = 0usize;

    for tag in &["article", "section", "div", "main", "p", "li", "span"] {
        let Ok(elements) = doc.select(tag) else {
            continue;
        };
        for el in elements {
            if hidden_reason(el.attr("style"), el.attr("class")).is_none() {
                continue;
            }

            if el
                .select("math, [data-mathml], .katex-mathml")
                .ok()
                .is_some_and(|matches| !matches.is_empty())
                || el.tag_name() == "math"
            {
                continue;
            }
            let text = el.text();
            let word_count = utils::count_words(&text);
            if word_count > best_word_count {
                best_word_count = word_count;
                best_html = el.outer_html();
            }
        }
    }

    if best_word_count == 0 {
        return None;
    }

    Some(build_single_block_extraction(
        doc,
        best_html,
        None,
        best_word_count as f64,
        None,
    ))
}

/// Extract content from schema.org structured data (final fallback in multi-pass retry).
///
/// Checks two sources in priority order:
/// 1. Microdata: `[itemprop="articleBody"]` or `[itemprop="text"]` elements
/// 2. JSON-LD: `articleBody` or `text` field in `<script type="application/ld+json">`
///
/// JSON-LD plain text is HTML-escaped and wrapped in a `<div>`.
pub(crate) fn extract_schema_org_article(doc: &Document) -> Option<ExtractedContent> {
    for selector in &[r#"[itemprop="articleBody"]"#, r#"[itemprop="text"]"#] {
        if let Ok(elements) = doc.select(selector)
            && let Some(el) = elements.first()
            && !el.text().trim().is_empty()
        {
            let content = el.outer_html();
            return Some(build_single_block_extraction(
                doc,
                content,
                Some(el.text()),
                100.0,
                None,
            ));
        }
    }

    let Ok(scripts) = doc.select(r#"script[type="application/ld+json"]"#) else {
        return None;
    };

    for script in scripts {
        let text = script.text();
        let Ok(value) = serde_json::from_str::<serde_json::Value>(text.trim()) else {
            continue;
        };

        for field in &["articleBody", "text"] {
            if let Some(body_text) = value.get(*field).and_then(|v| v.as_str()) {
                let trimmed = body_text.trim();
                if !trimmed.is_empty() {
                    let content = format!("<div>{}</div>", utils::escape_html(trimmed));
                    return Some(build_single_block_extraction(
                        doc,
                        content,
                        Some(trimmed.to_string()),
                        100.0,
                        None,
                    ));
                }
            }
        }
    }

    None
}

/// Extract content using site configuration with fallback to heuristics
pub fn extract_content_with_config(
    doc: &Document, config: &ExtractConfig, site_config: Option<&SiteConfig>,
) -> Result<ExtractedContent> {
    match site_config {
        Some(site_cfg) if !site_cfg.title.is_empty() || !site_cfg.body.is_empty() => {
            match extract_with_site_config(doc, site_cfg, config) {
                Ok(content) => Ok(content),
                Err(_) if site_cfg.should_autodetect() => extract_content(doc, config),
                Err(_) => Err(LectitoError::NoContent),
            }
        }
        _ => extract_content(doc, config),
    }
}

/// Extract content using explicit site configuration XPath expressions
fn extract_with_site_config(
    doc: &Document, site_config: &SiteConfig, config: &ExtractConfig,
) -> Result<ExtractedContent> {
    let html = doc.html().html();
    let body_content = site_config
        .extract_body_html(doc)?
        .or_else(|| site_config.extract_body(&html).ok().flatten())
        .ok_or(LectitoError::NoContent)?;

    let body_content = site_config.apply_strip_directives(&body_content)?;

    let body_content = if let Some(base_url) = doc.base_url() {
        preprocess::convert_relative_urls(&body_content, base_url)
    } else {
        body_content
    };

    let mut postprocess_config = config.postprocess.clone();
    if postprocess_config.base_url.is_none() {
        postprocess_config.base_url = doc.base_url().cloned();
    }
    let body_content = postprocess_html(&body_content, &postprocess_config);

    // TODO: we should use this
    let _title = site_config.extract_title(&html)?.or_else(|| doc.title());

    Ok(build_single_block_extraction(
        doc,
        body_content,
        None,
        100.0,
        Some("site-config"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_config_default() {
        let config = ExtractConfig::default();
        assert_eq!(config.min_score_threshold, 10.0);
        assert_eq!(config.max_top_candidates, 5);
        assert_eq!(config.char_threshold, 500);
        assert_eq!(config.max_elements, 1000);
        assert_eq!(config.sibling_threshold, 0.2);
        assert!(config.pre_score_selector_removal);
    }

    #[test]
    fn test_identify_candidates_simple_article() {
        let html = r#"
            <html>
                <body>
                    <div class="sidebar">Sidebar</div>
                    <article class="main-content">
                        <h1>Article Title</h1>
                        <p>This is a long paragraph with lots of content to ensure it meets the character threshold.
                        It continues with more text, more content, and even more text to increase the character count.
                        This should definitely qualify as a candidate with reasonable content density.</p>
                        <p>Another paragraph with substantial content. It has multiple sentences,
                        commas for density, and enough text to be considered meaningful content.
                        The scoring algorithm should recognize this as legitimate article content.</p>
                    </article>
                </body>
            </html>
        "#;

        let doc = Document::parse(html).unwrap();
        let config = ExtractConfig::default();
        let score_config = ScoreConfig::default();

        let candidates = identify_candidates(&doc, &config, &score_config);
        assert!(!candidates.is_empty());
        let has_article = candidates.iter().any(|c| c.element.tag_name() == "article");
        assert!(has_article);
    }

    #[test]
    fn test_select_top_candidate_threshold() {
        let html = r#"
            <html>
                <body>
                    <div class="sidebar" id="sidebar">
                        <p>Short sidebar text</p>
                    </div>
                    <article class="main-content" id="main">
                        <h1>Main Article Title</h1>
                        <p>This is a very long paragraph with extensive content. It contains multiple sentences,
                        commas, periods, and various punctuation marks. The purpose is to create a substantial
                        amount of text that will score well in the content density calculation. More text here,
                        more content, more sentences, more everything. This should definitely be the top candidate
                        with a score that exceeds the minimum threshold of 20 points.</p>
                    </article>
                </body>
            </html>
        "#;

        let doc = Document::parse(html).unwrap();
        let config = ExtractConfig::default();

        let result = extract_content(&doc, &config);
        assert!(result.is_ok());

        let extracted = result.unwrap();
        assert!(extracted.top_score >= config.min_score_threshold);
    }

    #[test]
    fn test_not_readable_error() {
        let html = r##"
            <html>
                <body>
                    <nav class="menu">
                        <a href="#">Link 1</a>
                        <a href="#">Link 2</a>
                        <a href="#">Link 3</a>
                        <a href="#">Link 4</a>
                        <a href="#">Link 5</a>
                        <a href="#">Link 6</a>
                    </nav>
                    <div class="sidebar">
                        This is a sidebar with some links and navigation.
                        <a href="#">Nav Link</a>
                        <a href="#">Another Link</a>
                        More sidebar content here.
                    </div>
                </body>
            </html>
        "##;

        let doc = Document::parse(html).unwrap();
        let config = ExtractConfig::default();

        let result = extract_content(&doc, &config);
        assert!(matches!(result, Err(LectitoError::NotReadable { .. })));

        if let Err(LectitoError::NotReadable { score, threshold }) = result {
            assert!(score < threshold);
        }
    }

    #[test]
    fn test_extract_content_with_siblings() {
        let html = r#"
            <html>
                <body>
                    <article class="content">
                        <h1>Main Article</h1>
                        <p class="lead">This is the lead paragraph with substantial content.
                        It has enough text to be considered, with commas, and meaningful content.</p>
                        <p>This is a supporting paragraph with content, text, and commas,
                        making it a good sibling candidate for extraction.</p>
                    </article>
                </body>
            </html>
        "#;

        let doc = Document::parse(html).unwrap();
        let config = ExtractConfig::default();

        let result = extract_content(&doc, &config);

        assert!(result.is_ok());
        let extracted = result.unwrap();

        assert!(!extracted.content.is_empty());
        assert!(extracted.top_score > 0.0);
    }

    #[test]
    fn test_aggregate_clustered_candidates_prefers_shared_ancestor() {
        let section_one = "Section one contains enough content, prose, and commas to score well. ".repeat(12);
        let section_two = "Section two contains enough content, prose, and commas to score well. ".repeat(12);
        let section_three = "Section three contains enough content, prose, and commas to score well. ".repeat(12);
        let html = format!(
            r#"
            <html>
                <body>
                    <article class="story-shell">
                        <section class="story-block"><p>{section_one}</p></section>
                        <section class="story-block"><p>{section_two}</p></section>
                        <section class="story-block"><p>{section_three}</p></section>
                    </article>
                </body>
            </html>
        "#,
        );

        let doc = Document::parse(&html).unwrap();
        let config = ExtractConfig::default();
        let extracted = extract_content(&doc, &config).unwrap();

        assert!(extracted.content.contains("<article"));
        assert!(extracted.content.contains("Section one contains enough content"));
        assert!(extracted.content.contains("Section two contains enough content"));
        assert!(extracted.content.contains("Section three contains enough content"));
    }

    #[test]
    fn test_shared_class_bonus_keeps_matching_sibling_section() {
        let html = r#"
            <html>
                <body>
                    <div class="story">
                        <section class="article-block">
                            <p>This is the first sibling block with enough text, commas, and structure to become the top candidate in extraction.</p>
                        </section>
                        <section class="article-block">
                            <p>This is the second sibling block with enough text to survive once the shared class bonus is applied during sibling selection.</p>
                        </section>
                    </div>
                </body>
            </html>
        "#;

        let doc = Document::parse(html).unwrap();
        let config = ExtractConfig { char_threshold: 50, sibling_threshold: 0.9, ..Default::default() };

        let extracted = extract_content(&doc, &config).unwrap();
        assert!(extracted.content.contains("first sibling block"));
        assert!(extracted.content.contains("second sibling block"));
    }

    #[test]
    fn test_candidate_score_propagation() {
        let html = r#"
            <html>
                <body>
                    <div class="container">
                        <article class="post">
                            <p>A long paragraph with content, text, and more content.
                            This should score reasonably well and propagate to parent containers.
                            More text to increase character count and content density.</p>
                        </article>
                    </div>
                </body>
            </html>
        "#;

        let doc = Document::parse(html).unwrap();
        let config = ExtractConfig::default();
        let score_config = ScoreConfig::default();

        let mut candidates = identify_candidates(&doc, &config, &score_config);
        let initial_count = candidates.len();

        let dom_tree = crate::build_dom_tree(&doc.as_string()).unwrap();
        propagate_scores(&mut candidates, &doc, &dom_tree);

        assert!(candidates.len() >= initial_count);
    }

    #[test]
    fn test_empty_document_error() {
        let html = r#"<html><body></body></html>"#;

        let doc = Document::parse(html).unwrap();
        let config = ExtractConfig::default();

        let result = extract_content(&doc, &config);

        assert!(matches!(result, Err(LectitoError::NoContent)));
    }

    #[test]
    fn test_extract_largest_hidden_subtree_finds_content() {
        let long_text = "word ".repeat(100);
        let html = format!(
            r#"<html><body>
                <nav>Short nav link</nav>
                <div style="display:none">
                    <p>{}</p>
                </div>
            </body></html>"#,
            long_text
        );

        let doc = Document::parse(&html).unwrap();
        let result = extract_largest_hidden_subtree(&doc);

        assert!(result.is_some());
        let extracted = result.unwrap();
        assert!(extracted.content.contains("word"));
        assert!(extracted.top_score > 0.0);
    }

    #[test]
    fn test_extract_largest_hidden_subtree_picks_largest() {
        let html = r#"<html><body>
            <div style="display:none">Short hidden text</div>
            <section style="visibility:hidden">
                This is a much longer hidden section with many more words and content
                that should be preferred over the shorter hidden div above.
            </section>
        </body></html>"#;

        let doc = Document::parse(html).unwrap();
        let result = extract_largest_hidden_subtree(&doc);

        assert!(result.is_some());
        let extracted = result.unwrap();
        assert!(extracted.content.contains("longer hidden section"));
    }

    #[test]
    fn test_extract_largest_hidden_subtree_detects_framework_hidden_classes() {
        let html = r#"<html><body>
            <div class="hidden">Short hidden text</div>
            <article class="lg:hidden">
                This hidden article contains enough words to win the fallback selection
                once framework hidden utilities are considered alongside inline styles.
            </article>
        </body></html>"#;

        let doc = Document::parse(html).unwrap();
        let result = extract_largest_hidden_subtree(&doc);

        assert!(result.is_some());
        let extracted = result.unwrap();
        assert!(extracted.content.contains("framework hidden utilities"));
    }

    #[test]
    fn test_extract_largest_hidden_subtree_empty_when_no_hidden() {
        let html = r#"<html><body>
            <div>Visible content</div>
            <p>More visible content</p>
        </body></html>"#;

        let doc = Document::parse(html).unwrap();
        let result = extract_largest_hidden_subtree(&doc);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_schema_org_microdata_article_body() {
        let html = r#"<html><body>
            <div itemprop="articleBody">
                <p>This is the article body content from microdata.</p>
                <p>It has multiple paragraphs with real text.</p>
            </div>
        </body></html>"#;

        let doc = Document::parse(html).unwrap();
        let result = extract_schema_org_article(&doc);

        assert!(result.is_some());
        let extracted = result.unwrap();
        assert!(extracted.content.contains("articleBody"));
        assert_eq!(extracted.top_score, 100.0);
    }

    #[test]
    fn test_extract_schema_org_json_ld_article_body() {
        let html = r#"<html>
        <head>
            <script type="application/ld+json">
            {
                "@context": "https://schema.org",
                "@type": "Article",
                "articleBody": "This is the full article body text extracted from JSON-LD structured data."
            }
            </script>
        </head>
        <body><nav>Just navigation</nav></body></html>"#;

        let doc = Document::parse(html).unwrap();
        let result = extract_schema_org_article(&doc);

        assert!(result.is_some());
        let extracted = result.unwrap();
        assert!(extracted.content.contains("full article body text"));
        assert_eq!(extracted.top_score, 100.0);
    }

    #[test]
    fn test_extract_schema_org_json_ld_text_field() {
        let html = r#"<html>
        <head>
            <script type="application/ld+json">
            {
                "@context": "https://schema.org",
                "@type": "WebPage",
                "text": "Article content from the text field of JSON-LD."
            }
            </script>
        </head>
        <body></body></html>"#;

        let doc = Document::parse(html).unwrap();
        let result = extract_schema_org_article(&doc);

        assert!(result.is_some());
        let extracted = result.unwrap();
        assert!(extracted.content.contains("Article content from the text field"));
    }

    #[test]
    fn test_extract_schema_org_returns_none_when_absent() {
        let html = r#"<html><body><p>No schema.org data here.</p></body></html>"#;

        let doc = Document::parse(html).unwrap();
        let result = extract_schema_org_article(&doc);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_content_removes_wikipedia_chrome_and_tail_sections() {
        let html = r#"
            <html>
                <body>
                    <div class="mw-parser-output">
                        <table class="infobox"><tr><td>Infobox</td></tr></table>
                        <h1>Rust</h1>
                        <span class="mw-editsection">edit</span>
                        <p>Rust is a systems programming language focused on safety and speed. It uses an ownership model, strong static typing, and explicit lifetimes to enforce memory safety without relying on a garbage collector.<sup class="reference">[1]</sup></p>
                        <p>The language was designed for predictable performance and fearless concurrency. Its compiler checks borrowing rules at compile time, which helps developers avoid entire classes of memory corruption and data race bugs.</p>
                        <h2>See also</h2>
                        <ul>
                            <li><a href="/wiki/Go_(programming_language)">Go</a></li>
                        </ul>
                        <h2>References</h2>
                        <ol class="references">
                            <li>Reference entry</li>
                        </ol>
                    </div>
                </body>
            </html>
        "#;

        let doc = Document::parse(html).unwrap();
        let extracted = extract_content(&doc, &ExtractConfig::default()).unwrap();

        assert!(extracted.content.contains("systems programming language"));
        assert!(!extracted.content.contains("Infobox"));
        assert!(!extracted.content.contains("mw-editsection"));
        assert!(!extracted.content.contains("[1]"));
        assert!(!extracted.content.contains("See also"));
        assert!(!extracted.content.contains("References"));
    }

    #[test]
    fn test_extract_content_removes_selector_matched_clutter_before_scoring() {
        let html = r#"
            <html>
                <body>
                    <article class="article-body">
                        <div class="newsletter-signup">Subscribe to our newsletter</div>
                        <p>This article explains ownership, borrowing, and lifetimes in enough detail to qualify as real article prose.</p>
                        <p>It also covers references, move semantics, and compile-time guarantees with substantial explanatory text.</p>
                    </article>
                </body>
            </html>
        "#;

        let doc = Document::parse(html).unwrap();
        let extracted = extract_content(&doc, &ExtractConfig::default()).unwrap();

        assert!(extracted.content.contains("ownership, borrowing, and lifetimes"));
        assert!(!extracted.content.contains("Subscribe to our newsletter"));
    }

    #[test]
    fn test_extract_content_prefers_specific_entry_point_inside_main() {
        let html = r#"
            <html>
                <body>
                    <main>
                        <header>
                            <h1>Integral</h1>
                            <p>94 languages</p>
                        </header>
                        <div class="mw-parser-output">
                            <p>Integrals are used to calculate area, volume, accumulation, and continuous change across many areas of mathematics and physics.</p>
                            <p>They connect antiderivatives to signed area and are one of the central concepts in calculus.</p>
                        </div>
                    </main>
                </body>
            </html>
        "#;

        let doc = Document::parse(html).unwrap();
        let extracted = extract_content(&doc, &ExtractConfig::default()).unwrap();

        assert!(extracted.content.contains("calculate area, volume, accumulation"));
        assert!(!extracted.content.contains("<main"));
    }
}
