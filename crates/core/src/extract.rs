use crate::dom_tree::DomTree;
use crate::parse::{Document, Element};
use crate::postprocess::{PostProcessConfig, postprocess_html};
use crate::scoring::{ScoreConfig, ScoreResult, calculate_score};
use crate::siteconfig::{SiteConfig, SiteConfigProcessing, SiteConfigXPath};
use crate::{LectitoError, Result, preprocess};

use regex::Regex;
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
#[derive(Debug, Clone)]
pub struct ExtractedContent {
    /// The main content element
    pub content: String,
    /// The top candidate score
    pub top_score: f64,
    /// Number of elements extracted
    pub element_count: usize,
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
const CANDIDATE_TAGS: &[&str] = &["div", "article", "section", "main", "p", "pre", "blockquote", "td"];

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
        "article" | "main" | "body" | "html" | "pre" | "code" | "math"
    ) || is_heading_tag(&tag)
    {
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
        .select("pre, code, math")
        .ok()
        .is_some_and(|els| !els.is_empty())
    {
        return false;
    }

    let text_len = element.text().chars().count();
    let paragraph_count = element.select("p").map(|els| els.len()).unwrap_or(0);
    let ld = crate::scoring::link_density(element);

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

fn prepare_scoring_document(doc: &Document, config: &ExtractConfig) -> Option<Document> {
    if !config.pre_score_selector_removal {
        return None;
    }

    let original_html = doc.as_string();
    let mut snippets = Vec::new();

    for selector in PRE_SCORE_EXACT_SELECTORS {
        if let Ok(elements) = doc.select(selector) {
            snippets.extend(elements.into_iter().map(|el| el.outer_html()));
        }
    }

    if let Ok(elements) = doc.select("*") {
        for element in elements {
            if should_remove_partial_selector_candidate(&element) {
                snippets.push(element.outer_html());
            }
        }
    }

    if snippets.is_empty() {
        return None;
    }

    let filtered_html = remove_html_snippets(original_html.clone(), snippets);
    if filtered_html == original_html {
        return None;
    }

    Document::parse_with_base_url(&filtered_html, doc.base_url().cloned()).ok()
}

fn count_words(text: &str) -> usize {
    text.split_whitespace().count()
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
            let words = count_words(&element.text());
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
        let key = if cand_html.len() > 200 {
            let truncated = truncate_at_char_boundary(&cand_html, 200);
            format!("{}-{}", candidate.element.tag_name(), truncated)
        } else {
            format!("{}-{}", candidate.element.tag_name(), cand_html)
        };
        processed_elements.insert(key);
    }

    for candidate in candidates.iter() {
        let candidate_score = candidate.score();
        let candidate_html = candidate.element.outer_html();
        let candidate_tag = candidate.element.tag_name();

        if let Some(parent_node) = dom_tree.get_parent_by_html(&candidate_html, &candidate_tag) {
            let parent_html = &parent_node.html;
            let parent_tag = &parent_node.tag_name;
            let parent_key = if parent_html.len() > 200 {
                let truncated = truncate_at_char_boundary(parent_html, 200);
                format!("{}-{}", parent_tag, truncated)
            } else {
                format!("{}-{}", parent_tag, parent_html)
            };

            if !processed_elements.contains(&parent_key)
                && let Ok(parent_elements) = doc.select(parent_tag)
            {
                for parent_elem in parent_elements {
                    if parent_elem.outer_html() == *parent_html {
                        let parent_score_result = calculate_score(&parent_elem, &score_config);
                        let boosted_score = parent_score_result.final_score + candidate_score / 2.0;

                        let mut boosted_result = parent_score_result.clone();
                        boosted_result.final_score = boosted_score;

                        additional_candidates.push(Candidate::new(parent_elem, boosted_result));
                        processed_elements.insert(parent_key);
                        break;
                    }
                }
            }

            if let Some(parent_id) = parent_node.parent_id
                && let Some(grandparent_node) = dom_tree.get_parent(parent_id)
            {
                let grandparent_html = &grandparent_node.html;
                let grandparent_tag = &grandparent_node.tag_name;
                let grandparent_key = if grandparent_html.len() > 200 {
                    let truncated = truncate_at_char_boundary(grandparent_html, 200);
                    format!("{}-{}", grandparent_tag, truncated)
                } else {
                    format!("{}-{}", grandparent_tag, grandparent_html)
                };

                if !processed_elements.contains(&grandparent_key)
                    && let Ok(grandparent_elements) = doc.select(grandparent_tag)
                {
                    for grandparent_elem in grandparent_elements {
                        if grandparent_elem.outer_html() == *grandparent_html {
                            let grandparent_score_result = calculate_score(&grandparent_elem, &score_config);
                            let boosted_score = grandparent_score_result.final_score + candidate_score / 3.0;

                            let mut boosted_result = grandparent_score_result.clone();
                            boosted_result.final_score = boosted_score;

                            additional_candidates.push(Candidate::new(grandparent_elem, boosted_result));
                            processed_elements.insert(grandparent_key);
                            break;
                        }
                    }
                }
            }
        }
    }

    candidates.extend(additional_candidates);
}

/// Select the top candidate from the list
///
/// Returns the highest scoring candidate if it meets the minimum threshold,
/// otherwise returns a NotReadable error.
fn select_top_candidate<'a>(candidates: &'a [Candidate<'a>], config: &ExtractConfig) -> Result<&'a Candidate<'a>> {
    if candidates.is_empty() {
        return Err(LectitoError::NoContent);
    }

    let top_candidate = candidates
        .iter()
        .max_by(|a, b| compare_candidates(a, b).unwrap_or(std::cmp::Ordering::Equal))
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

fn select_preferred_entry_candidate<'a>(
    entry_candidates: &'a [EntryPointCandidate<'a>], dom_tree: Option<&DomTree>,
) -> Option<&'a EntryPointCandidate<'a>> {
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
        return Some(top);
    };
    let Some(top_id) = node_id_for_element(&top.candidate.element, dom_tree) else {
        return Some(top);
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

    preferred.map(|(candidate, _)| candidate).or(Some(top))
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
    let scored_words = count_words(&scored_candidate.element.text());
    scored_words >= MIN_ENTRY_POINT_WORDS && scored_words * 10 >= entry_words * 6
}

fn entry_candidate_meets_threshold(entry_candidate: &EntryPointCandidate<'_>, config: &ExtractConfig) -> bool {
    let tag = entry_candidate.candidate.element.tag_name();
    tag != "body" || entry_candidate.content_score >= config.min_score_threshold
}

fn looks_like_table_layout(table: &Element<'_>) -> bool {
    let width = table
        .attr("width")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    if width > 400
        || table
            .attr("align")
            .is_some_and(|value| value.eq_ignore_ascii_case("center"))
    {
        return true;
    }

    let attrs = selector_attrs(table);
    if attrs.contains("content") || attrs.contains("article") {
        return true;
    }

    table.select("tr").unwrap_or_default().into_iter().any(|row| {
        let cells = row.select("td, th").unwrap_or_default();
        cells.len() >= 2 && cells.iter().any(|cell| cell.attr("width").is_some())
    })
}

fn select_table_layout_candidate<'a>(doc: &'a Document, score_config: &ScoreConfig) -> Option<Candidate<'a>> {
    let tables = doc.select("table").ok()?;
    if !tables.iter().any(looks_like_table_layout) {
        return None;
    }

    let mut candidates = Vec::new();

    for selector in ["td", "table"] {
        let Ok(elements) = doc.select(selector) else {
            continue;
        };
        for element in elements {
            let words = count_words(&element.text());
            if words < MIN_ENTRY_POINT_WORDS {
                continue;
            }

            let link_density = crate::scoring::link_density(&element);
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
    let dom_tree = match dom_tree {
        Some(tree) => tree,
        None => return siblings,
    };
    let top_parent_id = parent_id_for(&top_candidate.element, dom_tree);
    if top_parent_id.is_none() {
        return siblings;
    }

    for candidate in candidates {
        if candidate.score() >= top_score * config.sibling_threshold {
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
                        let link_density = crate::scoring::link_density(&candidate.element);
                        if link_density < 0.25 {
                            siblings.push(candidate.element.clone());
                        }
                    }
                } else {
                    siblings.push(candidate.element.clone());
                }
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
            let link_density = crate::scoring::link_density(&header);
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
            let link_density = crate::scoring::link_density(heading);
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
    let filtered_doc = prepare_scoring_document(doc, config);
    let mut working_doc = filtered_doc.as_ref().unwrap_or(doc);
    let score_config = ScoreConfig::default();
    let mut entry_candidates = identify_entry_point_candidates(working_doc, &score_config);

    let mut candidates = identify_candidates(working_doc, config, &score_config);

    if candidates.is_empty() && filtered_doc.is_some() {
        working_doc = doc;
        entry_candidates = identify_entry_point_candidates(working_doc, &score_config);
        candidates = identify_candidates(working_doc, config, &score_config);
    }

    let dom_tree = crate::build_dom_tree(&working_doc.as_string()).ok();
    if let Some(tree) = dom_tree.as_ref() {
        propagate_scores(&mut candidates, working_doc, tree);
    }

    candidates.sort_by(|a, b| b.score().partial_cmp(&a.score()).unwrap_or(std::cmp::Ordering::Equal));
    candidates.truncate(config.max_top_candidates);
    let table_layout_candidate = select_table_layout_candidate(working_doc, &score_config);
    let scored_top_candidate = select_top_candidate(&candidates, config).ok();
    let preferred_entry_candidate = select_preferred_entry_candidate(&entry_candidates, dom_tree.as_ref())
        .filter(|candidate| candidate.score() >= config.min_score_threshold)
        .filter(|candidate| entry_candidate_meets_threshold(candidate, config));

    let top_candidate = match (
        preferred_entry_candidate,
        scored_top_candidate,
        table_layout_candidate.as_ref(),
    ) {
        (Some(entry_candidate), Some(scored_candidate), _)
            if should_prefer_scored_descendant(entry_candidate, scored_candidate, dom_tree.as_ref()) =>
        {
            scored_candidate
        }
        (Some(entry_candidate), _, _) => &entry_candidate.candidate,
        (None, Some(scored_candidate), _) => scored_candidate,
        (None, None, Some(table_candidate)) if table_candidate.score() >= config.min_score_threshold => table_candidate,
        _ => select_top_candidate(&candidates, config)?,
    };
    let siblings = select_siblings(working_doc, top_candidate, &candidates, config, dom_tree.as_ref());
    let mut leading = Vec::new();
    let mut trailing = Vec::new();

    for sibling in siblings {
        match sibling.tag_name().as_str() {
            "header" | "h1" => leading.push(sibling),
            _ => trailing.push(sibling),
        }
    }

    let mut content = String::new();
    for sibling in &leading {
        if !content.is_empty() {
            content.push('\n');
        }
        content.push_str(&sibling.outer_html());
    }

    if !content.is_empty() {
        content.push('\n');
    }
    content.push_str(&top_candidate.element.outer_html());

    for sibling in &trailing {
        content.push('\n');
        content.push_str(&sibling.outer_html());
    }

    if let Some(title) = doc.title() {
        let has_h1 = content.contains("<h1");
        let content_text = Document::parse(&content).map(|d| d.text_content()).unwrap_or_default();
        if !has_h1 && !content_text.contains(&title) {
            let safe_title = escape_html(&title);
            content = format!("<h1>{}</h1>\n{}", safe_title, content);
        }
    }

    let content = postprocess_html(&content, &config.postprocess);

    let element_count = 1 + leading.len() + trailing.len();

    Ok(ExtractedContent { content, top_score: top_candidate.score(), element_count })
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

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
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

/// Extract content from the largest hidden element subtree (Pass 3 of multi-pass retry).
///
/// Scans for elements hidden via inline `display:none` or `visibility:hidden` styles
/// and returns the subtree with the most text content. This targets pages that hide
/// their main content until JavaScript runs.
pub(crate) fn extract_largest_hidden_subtree(doc: &Document) -> Option<ExtractedContent> {
    let hidden_pattern = regex::Regex::new(r"(?i)(display\s*:\s*none|visibility\s*:\s*hidden)").unwrap();

    let mut best_html = String::new();
    let mut best_word_count = 0usize;

    for tag in &["article", "section", "div", "main", "p", "li", "span"] {
        let Ok(elements) = doc.select(tag) else {
            continue;
        };
        for el in elements {
            let style = el.attr("style").unwrap_or("");
            if !hidden_pattern.is_match(style) {
                continue;
            }
            let text = el.text();
            let word_count = text.split_whitespace().count();
            if word_count > best_word_count {
                best_word_count = word_count;
                best_html = el.outer_html();
            }
        }
    }

    if best_word_count == 0 {
        return None;
    }

    Some(ExtractedContent { content: best_html, top_score: best_word_count as f64, element_count: 1 })
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
            return Some(ExtractedContent { content: el.outer_html(), top_score: 100.0, element_count: 1 });
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
                    return Some(ExtractedContent {
                        content: format!("<div>{}</div>", escape_html(trimmed)),
                        top_score: 100.0,
                        element_count: 1,
                    });
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
    doc: &Document, site_config: &SiteConfig, _config: &ExtractConfig,
) -> Result<ExtractedContent> {
    let html = doc.html().html();

    let body_content = 'extracted: {
        for xpath in &site_config.body {
            if xpath.contains("[@id='")
                && let Some(start) = xpath.find("[@id='")
                && let Some(end) = xpath[start + 6..].find("']")
            {
                let id = &xpath[start + 6..start + 6 + end];
                let selector_str = format!("#{}", id);
                match scraper::Selector::parse(&selector_str) {
                    Ok(_selector) => {
                        if let Ok(elements) = doc.select(&selector_str)
                            && !elements.is_empty()
                        {
                            break 'extracted elements.iter().map(|el| el.outer_html()).collect::<Vec<_>>().join("\n");
                        }
                    }
                    Err(_) => continue,
                }
            }

            if xpath.contains("[@class='")
                && let Some(start) = xpath.find("[@class='")
                && let Some(end) = xpath[start + 8..].find("']")
            {
                let class = &xpath[start + 8..start + 8 + end];
                if let Some(tag_end) = xpath[2..].find('[') {
                    let tag = &xpath[2..2 + tag_end];
                    let selector_str = format!("{}.{}", tag, class);
                    match scraper::Selector::parse(&selector_str) {
                        Ok(_) => {
                            if let Ok(elements) = doc.select(&selector_str)
                                && !elements.is_empty()
                            {
                                break 'extracted elements
                                    .iter()
                                    .map(|el| el.outer_html())
                                    .collect::<Vec<_>>()
                                    .join("\n");
                            }
                        }
                        Err(_) => continue,
                    }
                }
            }
        }

        if let Some(body) = site_config.extract_body(&html)? {
            body
        } else {
            return Err(LectitoError::NoContent);
        }
    };

    let body_content = site_config.apply_strip_directives(&body_content)?;

    let body_content = if let Some(base_url) = doc.base_url() {
        preprocess::convert_relative_urls(&body_content, base_url)
    } else {
        body_content
    };

    // TODO: we should use this
    let _title = site_config.extract_title(&html)?.or_else(|| doc.title());

    let element_count = 1;
    let top_score = 100.0;

    Ok(ExtractedContent { content: body_content, element_count, top_score })
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
