use kuchiki::NodeRef;

use super::diagnostics::RecoveryDiagnostic;
use super::regexes::RegexPattern;
use super::{dom, patterns};

pub fn recover_html_snapshot(html: &str) -> (String, RecoveryDiagnostic) {
    let mut flattened = 0;
    let html = RegexPattern::ShadowTemplateHtml
        .to_regex()
        .replace_all(html, |captures: &regex::Captures<'_>| {
            flattened += 1;
            captures
                .name("body")
                .map(|body| body.as_str())
                .unwrap_or_default()
                .to_string()
        })
        .into_owned();
    (
        html,
        RecoveryDiagnostic { shadow_roots_flattened: flattened, mobile_rules_applied: 0 },
    )
}

pub fn recover(document: &NodeRef, mobile_viewport_width: Option<usize>) -> RecoveryDiagnostic {
    let mut diagnostic = RecoveryDiagnostic {
        shadow_roots_flattened: flatten_declarative_shadow_dom(document),
        mobile_rules_applied: 0,
    };
    if let Some(width) = mobile_viewport_width {
        diagnostic.mobile_rules_applied = apply_mobile_display_rules(document, width);
    }
    diagnostic
}

fn flatten_declarative_shadow_dom(document: &NodeRef) -> usize {
    let mut flattened = 0;
    for template in dom::select_nodes(document, r#"template[shadowrootmode], template[shadowroot]"#) {
        let children: Vec<_> = template.children().collect();
        if children.is_empty() {
            continue;
        }
        for child in children {
            template.insert_before(child);
        }
        template.detach();
        flattened += 1;
    }
    flattened
}

fn apply_mobile_display_rules(document: &NodeRef, viewport_width: usize) -> usize {
    let mut applied = 0;
    for style in dom::select_nodes(document, "style") {
        let css = style.text_contents();
        for media in RegexPattern::MobileMediaBlock.to_regex().captures_iter(&css) {
            let max_width = media
                .get(1)
                .and_then(|width| width.as_str().parse::<usize>().ok())
                .unwrap_or(0);
            if max_width == 0 || viewport_width > max_width {
                continue;
            }
            let Some(body_start) = media.get(0).map(|matched| matched.end()) else {
                continue;
            };
            let body = &css[body_start..];
            for rule in RegexPattern::CssRule.to_regex().captures_iter(body) {
                let display = rule
                    .name("body")
                    .and_then(|body| RegexPattern::DisplayDecl.to_regex().captures(body.as_str()))
                    .and_then(|captures| {
                        captures
                            .name("display")
                            .map(|display| display.as_str().to_ascii_lowercase())
                    });
                let Some(display) = display else {
                    continue;
                };
                if display == "none" {
                    continue;
                }
                let Some(selectors) = rule.name("selectors").map(|selectors| selectors.as_str()) else {
                    continue;
                };
                for selector in selectors
                    .split(',')
                    .map(str::trim)
                    .filter(|selector| safe_selector(selector))
                {
                    for node in dom::select_nodes(document, selector) {
                        if remove_display_none(&node) {
                            applied += 1;
                        }
                    }
                }
            }
        }
    }
    applied
}

fn safe_selector(selector: &str) -> bool {
    !selector.is_empty()
        && selector.len() < 120
        && !selector.contains(':')
        && selector.chars().all(|ch| {
            ch.is_ascii_alphanumeric() || matches!(ch, '#' | '.' | '-' | '_' | '[' | ']' | '=' | '"' | '\'' | ' ')
        })
}

fn remove_display_none(node: &NodeRef) -> bool {
    let Some(style) = dom::attr(node, "style") else {
        return false;
    };
    if !patterns::has_display_none(Some(&style)) {
        return false;
    }

    let declarations: Vec<_> = style
        .split(';')
        .filter(|declaration| {
            let Some((property, _)) = declaration.split_once(':') else {
                return !declaration.trim().is_empty();
            };
            !property.trim().eq_ignore_ascii_case("display")
        })
        .map(str::trim)
        .filter(|declaration| !declaration.is_empty())
        .collect();
    if declarations.is_empty() {
        dom::remove_attr(node, "style");
    } else {
        dom::set_attr(node, "style", &declarations.join("; "));
    }
    true
}
