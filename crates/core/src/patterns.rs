use super::regexes::RegexPattern;

pub const TAGS_TO_SCORE: &[&str] = &["section", "h2", "h3", "h4", "h5", "h6", "p", "td", "pre"];

pub const DEPRECATED_SIZE_ATTRIBUTE_ELEMS: &[&str] = &["table", "th", "td", "hr", "pre"];

pub const DEFAULT_CLASSES_TO_PRESERVE: &[&str] = &["page"];

pub const PRESENTATIONAL_ATTRIBUTES: &[&str] = &[
    "align",
    "background",
    "bgcolor",
    "border",
    "cellpadding",
    "cellspacing",
    "frame",
    "hspace",
    "rules",
    "style",
    "valign",
    "vspace",
];

pub fn normalize_spaces(text: &str) -> String {
    RegexPattern::NormalizeWhitespace
        .to_regex()
        .replace_all(text, " ")
        .into_owned()
}

pub fn has_display_none(style: Option<&str>) -> bool {
    style
        .unwrap_or_default()
        .split(';')
        .filter_map(|declaration| declaration.split_once(':'))
        .any(|(property, value)| {
            property.trim().eq_ignore_ascii_case("display") && value.trim().eq_ignore_ascii_case("none")
        })
}

pub fn selector(pattern: &str) -> scraper::Selector {
    scraper::Selector::parse(pattern).expect("internal selector should parse")
}
