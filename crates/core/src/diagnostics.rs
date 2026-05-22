use serde::Serialize;

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct ExtractionDiagnostics {
    pub content_selector: Option<ContentSelectorDiagnostic>,
    pub site_rule: Option<SiteRuleDiagnostic>,
    pub attempts: Vec<AttemptDiagnostic>,
    pub selected_attempt: Option<usize>,
    pub outcome: ExtractionOutcome,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionOutcome {
    Accepted,
    BestAttempt,
    #[default]
    NoContent,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ExtractionReport {
    pub article: Option<crate::Article>,
    pub diagnostics: ExtractionDiagnostics,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ContentSelectorDiagnostic {
    pub selector: String,
    pub matched: bool,
    pub selected: Option<NodeDiagnostic>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct SiteRuleDiagnostic {
    pub name: String,
    pub source: SiteRuleSource,
    pub matched_by: SiteRuleMatch,
    pub roots: Vec<String>,
    pub removals: usize,
    pub text_len: usize,
    pub accepted: bool,
    pub fallback_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SiteRuleSource {
    DeclarativeProfile,
    CodeExtractor,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct SiteRuleMatch {
    pub host: String,
    pub path_prefix: Option<String>,
    pub bundled: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct AttemptDiagnostic {
    pub index: usize,
    pub flags: FlagDiagnostic,
    pub candidate_count: usize,
    pub candidates: Vec<CandidateDiagnostic>,
    pub entry_points: Vec<CandidateDiagnostic>,
    pub selected_root: Option<NodeDiagnostic>,
    pub cleanup: Option<CleanupDiagnostic>,
    pub recovery: RecoveryDiagnostic,
    pub text_len: usize,
    pub accepted: bool,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq)]
pub struct FlagDiagnostic {
    pub strip_unlikely: bool,
    pub weight_classes: bool,
    pub clean_conditionally: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct CandidateDiagnostic {
    pub node: NodeDiagnostic,
    pub score: f64,
    pub selected_by: CandidateSelection,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CandidateSelection {
    CandidateScoring,
    EntryPointPreselection,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct NodeDiagnostic {
    pub selector: String,
    pub tag: String,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub text_len: usize,
    pub link_density: f64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct CleanupDiagnostic {
    pub roots: Vec<String>,
    pub text_len_before: usize,
    pub text_len_after: usize,
    pub element_count_before: usize,
    pub element_count_after: usize,
    pub removed_elements: usize,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct RecoveryDiagnostic {
    pub shadow_roots_flattened: usize,
    pub mobile_rules_applied: usize,
}
