use serde::Serialize;

/// Details about how extraction selected, cleaned, and accepted article roots.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct ExtractionDiagnostics {
    /// Result of applying a caller-provided `content_selector`.
    pub content_selector: Option<ContentSelectorDiagnostic>,
    /// Result of applying a bundled or caller-provided site profile.
    pub site_rule: Option<SiteRuleDiagnostic>,
    /// Generic extraction attempts tried after profile and selector handling.
    pub attempts: Vec<AttemptDiagnostic>,
    /// Index into `attempts` for the selected generic extraction attempt.
    pub selected_attempt: Option<usize>,
    /// Final extraction outcome.
    pub outcome: ExtractionOutcome,
}

/// Final status for an extraction report.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionOutcome {
    /// Extraction met the configured acceptance thresholds.
    Accepted,
    /// Extraction returned the best available attempt below normal thresholds.
    BestAttempt,
    /// Extraction found no usable article content.
    #[default]
    NoContent,
}

/// Article output plus diagnostics for the decisions made during extraction.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ExtractionReport {
    /// Extracted article, if any content was accepted.
    pub article: Option<crate::Article>,
    /// Root selection, cleanup, and fallback diagnostics.
    pub diagnostics: ExtractionDiagnostics,
}

/// Diagnostics for a caller-provided content selector.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ContentSelectorDiagnostic {
    /// CSS selector supplied by the caller.
    pub selector: String,
    /// Whether the selector matched at least one node.
    pub matched: bool,
    /// Node selected for extraction when a match was usable.
    pub selected: Option<NodeDiagnostic>,
}

/// Diagnostics for a matched site-specific extraction rule.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct SiteRuleDiagnostic {
    /// Profile or extractor name.
    pub name: String,
    /// Where the rule came from.
    pub source: SiteRuleSource,
    /// Host and path match that selected this rule.
    pub matched_by: SiteRuleMatch,
    /// CSS-like selectors for roots selected by the rule.
    pub roots: Vec<String>,
    /// Number of elements removed by the rule before cleanup.
    pub removals: usize,
    /// Text length after rule extraction.
    pub text_len: usize,
    /// Whether the rule output met acceptance thresholds.
    pub accepted: bool,
    /// Reason generic extraction was used after a weak rule result.
    pub fallback_reason: Option<String>,
}

/// Source of a site-specific extraction rule.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SiteRuleSource {
    /// A TOML profile selected the content.
    DeclarativeProfile,
    /// A Rust extractor handled the site.
    CodeExtractor,
}

/// Host and path rule that matched a page URL.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct SiteRuleMatch {
    /// Hostname matched by the rule.
    pub host: String,
    /// Optional path prefix required by the rule.
    pub path_prefix: Option<String>,
    /// Whether the matching rule is bundled with Lectito.
    pub bundled: bool,
}

/// Diagnostics for one generic extraction attempt.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct AttemptDiagnostic {
    /// Attempt index in retry order.
    pub index: usize,
    /// Cleanup and scoring flags used for this attempt.
    pub flags: FlagDiagnostic,
    /// Number of scored candidate roots.
    pub candidate_count: usize,
    /// Highest-scoring generic candidate roots.
    pub candidates: Vec<CandidateDiagnostic>,
    /// Candidate roots found through article-like entry points.
    pub entry_points: Vec<CandidateDiagnostic>,
    /// Root selected for cleanup and serialization.
    pub selected_root: Option<NodeDiagnostic>,
    /// Cleanup result for the selected root.
    pub cleanup: Option<CleanupDiagnostic>,
    /// Content recovered from hidden or alternate markup before scoring.
    pub recovery: RecoveryDiagnostic,
    /// Extracted text length after cleanup.
    pub text_len: usize,
    /// Whether this attempt met acceptance thresholds.
    pub accepted: bool,
}

/// Generic extraction flags used for an attempt.
#[derive(Clone, Copy, Debug, Serialize, PartialEq)]
pub struct FlagDiagnostic {
    /// Whether unlikely page chrome was stripped before scoring.
    pub strip_unlikely: bool,
    /// Whether positive and negative class names affected scoring.
    pub weight_classes: bool,
    /// Whether low-value child nodes were removed conditionally.
    pub clean_conditionally: bool,
}

/// Diagnostics for one candidate article root.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct CandidateDiagnostic {
    /// Candidate node summary.
    pub node: NodeDiagnostic,
    /// Readability score assigned to the candidate.
    pub score: f64,
    /// How this candidate entered the selection set.
    pub selected_by: CandidateSelection,
}

/// Reason a node was considered as an article root.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CandidateSelection {
    /// The node was found by generic readability scoring.
    CandidateScoring,
    /// The node matched an article-like entry point before scoring.
    EntryPointPreselection,
}

/// Stable summary of a DOM node used in diagnostics.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct NodeDiagnostic {
    /// CSS-like selector for the node.
    pub selector: String,
    /// HTML tag name.
    pub tag: String,
    /// Element id, if present.
    pub id: Option<String>,
    /// Element classes.
    pub classes: Vec<String>,
    /// Visible text length under the node.
    pub text_len: usize,
    /// Link text ratio under the node.
    pub link_density: f64,
}

/// Cleanup measurements for a selected article root.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct CleanupDiagnostic {
    /// Root selectors cleaned for output.
    pub roots: Vec<String>,
    /// Text length before cleanup.
    pub text_len_before: usize,
    /// Text length after cleanup.
    pub text_len_after: usize,
    /// Element count before cleanup.
    pub element_count_before: usize,
    /// Element count after cleanup.
    pub element_count_after: usize,
    /// Number of elements removed during cleanup.
    pub removed_elements: usize,
}

/// Content recovery performed before scoring.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct RecoveryDiagnostic {
    /// Number of declarative shadow roots flattened into normal markup.
    pub shadow_roots_flattened: usize,
    /// Number of mobile-only style rules applied to recover hidden content.
    pub mobile_rules_applied: usize,
}
