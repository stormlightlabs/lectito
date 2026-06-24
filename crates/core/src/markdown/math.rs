use kuchiki::NodeRef;

use super::{RenderContext, render_children};
use crate::{dom, patterns};

pub(super) fn render_math(node: &NodeRef, _ctx: RenderContext) -> Option<String> {
    let latex = latex_for_node(node)?;
    let latex = normalize_latex(&latex);
    if latex.is_empty() {
        return None;
    }

    if is_display_math(node) {
        Some(format!("\n\n$$\n{latex}\n$$\n\n"))
    } else {
        Some(format!("${latex}$"))
    }
}

pub(super) fn has_math(node: &NodeRef) -> bool {
    !dom::select_nodes(
        node,
        "math, mjx-container, .katex, .math, .mwe-math-element, img[alttext], img[data-latex], span[data-latex], script[type]",
    )
    .is_empty()
}

fn latex_for_node(node: &NodeRef) -> Option<String> {
    latex_from_attrs(node)
        .or_else(|| latex_from_katex_annotation(node))
        .or_else(|| latex_from_script(node))
        .or_else(|| latex_from_mathml(node))
        .or_else(|| latex_from_math_descendant(node))
}

fn latex_from_attrs(node: &NodeRef) -> Option<String> {
    for attr in ["data-latex", "data-tex", "alttext", "alt"] {
        let value = dom::attr(node, attr)?;
        let value = strip_math_delimiters(value.trim());
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

fn latex_from_katex_annotation(node: &NodeRef) -> Option<String> {
    dom::select_nodes(node, "annotation")
        .into_iter()
        .find_map(|annotation| {
            let encoding = dom::attr(&annotation, "encoding")
                .unwrap_or_default()
                .to_ascii_lowercase();
            if encoding.contains("tex") || encoding.contains("latex") {
                let text = annotation.text_contents();
                let latex = strip_math_delimiters(text.trim());
                if latex.is_empty() { None } else { Some(latex.to_string()) }
            } else {
                None
            }
        })
}

fn latex_from_script(node: &NodeRef) -> Option<String> {
    if dom::node_name(node) != "script" {
        return None;
    }
    let script_type = dom::attr(node, "type").unwrap_or_default().to_ascii_lowercase();
    if !script_type.contains("math/tex") && !script_type.contains("latex") {
        return None;
    }
    let text = node.text_contents();
    let latex = strip_math_delimiters(text.trim());
    if latex.is_empty() { None } else { Some(latex.to_string()) }
}

fn latex_from_math_descendant(node: &NodeRef) -> Option<String> {
    dom::select_nodes(node, "math")
        .into_iter()
        .find_map(|math| latex_from_mathml(&math))
}

fn latex_from_mathml(node: &NodeRef) -> Option<String> {
    if dom::node_name(node) != "math" {
        return None;
    }
    let latex = render_mathml_children(node);
    if latex.is_empty() { None } else { Some(latex) }
}

fn is_display_math(node: &NodeRef) -> bool {
    if matches!(dom::attr(node, "display").as_deref(), Some("block" | "true")) {
        return true;
    }

    if dom::attr(node, "type")
        .unwrap_or_default()
        .to_ascii_lowercase()
        .contains("mode=display")
    {
        return true;
    }

    let class_id = dom::class_id_string(node).to_ascii_lowercase();
    if class_id.contains("display") || class_id.contains("block") {
        return true;
    }

    dom::select_nodes(node, "math, mjx-container")
        .into_iter()
        .any(|child| dom::node_id(&child) != dom::node_id(node) && is_display_math(&child))
}

fn render_mathml_children(node: &NodeRef) -> String {
    join_latex(node.children().map(|child| render_mathml(&child)).collect())
}

fn render_mathml(node: &NodeRef) -> String {
    if let Some(text) = node.as_text() {
        return math_text(&text.borrow());
    }

    match dom::node_name(node).as_str() {
        "math" | "mrow" | "mstyle" | "mpadded" | "menclose" => render_mathml_children(node),
        "semantics" => node
            .children()
            .find(|child| !matches!(dom::node_name(child).as_str(), "annotation" | "annotation-xml"))
            .map(|child| render_mathml(&child))
            .unwrap_or_default(),
        "mi" | "mn" | "mo" | "mtext" => math_text(&node.text_contents()),
        "msup" => scripted(node, "^", 0, 1),
        "msub" => scripted(node, "_", 0, 1),
        "msubsup" => {
            let children = meaningful_children(node);
            if children.len() >= 3 {
                format!(
                    "{}_{{{}}}^{{{}}}",
                    latex_group(render_mathml(&children[0])),
                    render_mathml(&children[1]),
                    render_mathml(&children[2])
                )
            } else {
                render_mathml_children(node)
            }
        }
        "mfrac" => {
            let children = meaningful_children(node);
            if children.len() >= 2 {
                format!(
                    "\\frac{{{}}}{{{}}}",
                    render_mathml(&children[0]),
                    render_mathml(&children[1])
                )
            } else {
                render_mathml_children(node)
            }
        }
        "msqrt" => format!("\\sqrt{{{}}}", render_mathml_children(node)),
        "mroot" => {
            let children = meaningful_children(node);
            if children.len() >= 2 {
                format!(
                    "\\sqrt[{}]{{{}}}",
                    render_mathml(&children[1]),
                    render_mathml(&children[0])
                )
            } else {
                render_mathml_children(node)
            }
        }
        "mfenced" => {
            let open = dom::attr(node, "open").unwrap_or_else(|| "(".to_string());
            let close = dom::attr(node, "close").unwrap_or_else(|| ")".to_string());
            format!("\\left{} {} \\right{}", open, render_mathml_children(node), close)
        }
        "mtable" => render_table_mathml(node),
        "mtr" | "mlabeledtr" => {
            join_latex(node.children().map(|child| render_mathml(&child)).collect::<Vec<_>>()).replace("  ", " ")
        }
        "mtd" => render_mathml_children(node),
        "mover" => mover(node),
        "munder" => underscript(node),
        "munderover" => undersuperscript(node),
        "mspace" | "annotation" | "annotation-xml" => String::new(),
        _ => {
            let rendered = render_mathml_children(node);
            if rendered.is_empty() {
                patterns::normalize_spaces(render_children(node, RenderContext { in_pre: false, list_depth: 0 }).trim())
            } else {
                rendered
            }
        }
    }
}

fn scripted(node: &NodeRef, marker: &str, base_index: usize, script_index: usize) -> String {
    let children = meaningful_children(node);
    if children.len() > script_index {
        format!(
            "{}{}{{{}}}",
            latex_group(render_mathml(&children[base_index])),
            marker,
            render_mathml(&children[script_index])
        )
    } else {
        render_mathml_children(node)
    }
}

fn mover(node: &NodeRef) -> String {
    let children = meaningful_children(node);
    if children.len() < 2 {
        return render_mathml_children(node);
    }
    let base = render_mathml(&children[0]);
    match render_mathml(&children[1]).trim() {
        "˙" | "." => format!("\\dot{{{base}}}"),
        "¨" => format!("\\ddot{{{base}}}"),
        "¯" | "―" | "-" => format!("\\bar{{{base}}}"),
        "→" | "\\to" => format!("\\vec{{{base}}}"),
        over => format!("\\overset{{{over}}}{{{base}}}"),
    }
}

fn underscript(node: &NodeRef) -> String {
    let children = meaningful_children(node);
    if children.len() >= 2 {
        format!(
            "\\underset{{{}}}{{{}}}",
            render_mathml(&children[1]),
            render_mathml(&children[0])
        )
    } else {
        render_mathml_children(node)
    }
}

fn undersuperscript(node: &NodeRef) -> String {
    let children = meaningful_children(node);
    if children.len() >= 3 {
        format!(
            "\\overset{{{}}}{{\\underset{{{}}}{{{}}}}}",
            render_mathml(&children[2]),
            render_mathml(&children[1]),
            render_mathml(&children[0])
        )
    } else {
        render_mathml_children(node)
    }
}

fn render_table_mathml(node: &NodeRef) -> String {
    dom::select_nodes(node, "mtr, mlabeledtr")
        .into_iter()
        .map(|row| {
            row.children()
                .filter(|child| dom::node_name(child) == "mtd")
                .map(|cell| render_mathml(&cell))
                .filter(|cell| !cell.trim().is_empty())
                .collect::<Vec<_>>()
                .join(" & ")
        })
        .filter(|row| !row.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" \\\\ ")
}

fn meaningful_children(node: &NodeRef) -> Vec<NodeRef> {
    node.children()
        .filter(|child| {
            child.as_element().is_some() || child.as_text().is_some_and(|text| !text.borrow().trim().is_empty())
        })
        .collect()
}

fn join_latex(parts: Vec<String>) -> String {
    normalize_latex(
        &parts
            .into_iter()
            .filter(|part| !part.trim().is_empty())
            .collect::<Vec<_>>()
            .join(" "),
    )
}

fn latex_group(value: String) -> String {
    let value = normalize_latex(&value);
    if value.chars().count() == 1 || value.starts_with('\\') { value } else { format!("{{{value}}}") }
}

fn math_text(value: &str) -> String {
    let value = patterns::normalize_spaces(value.trim());
    match value.as_str() {
        "−" => "-".to_string(),
        "±" => "\\pm".to_string(),
        "∓" => "\\mp".to_string(),
        "×" => "\\times".to_string(),
        "⋅" | "·" => "\\cdot".to_string(),
        "÷" => "\\div".to_string(),
        "≠" => "\\ne".to_string(),
        "≤" => "\\le".to_string(),
        "≥" => "\\ge".to_string(),
        "≈" => "\\approx".to_string(),
        "∞" => "\\infty".to_string(),
        "∂" => "\\partial".to_string(),
        "∇" => "\\nabla".to_string(),
        "→" => "\\to".to_string(),
        "←" => "\\leftarrow".to_string(),
        "↔" => "\\leftrightarrow".to_string(),
        "∈" => "\\in".to_string(),
        "∉" => "\\notin".to_string(),
        "∑" => "\\sum".to_string(),
        "∏" => "\\prod".to_string(),
        "∫" => "\\int".to_string(),
        "π" => "\\pi".to_string(),
        "α" => "\\alpha".to_string(),
        "β" => "\\beta".to_string(),
        "γ" => "\\gamma".to_string(),
        "δ" => "\\delta".to_string(),
        "ε" => "\\epsilon".to_string(),
        "θ" => "\\theta".to_string(),
        "λ" => "\\lambda".to_string(),
        "μ" => "\\mu".to_string(),
        "σ" => "\\sigma".to_string(),
        "φ" => "\\phi".to_string(),
        "ω" => "\\omega".to_string(),
        _ => value,
    }
}

fn strip_math_delimiters(value: &str) -> &str {
    value
        .trim()
        .trim_start_matches("\\(")
        .trim_end_matches("\\)")
        .trim_start_matches("\\[")
        .trim_end_matches("\\]")
        .trim_start_matches("$$")
        .trim_end_matches("$$")
        .trim_start_matches('$')
        .trim_end_matches('$')
        .trim()
}

fn normalize_latex(value: &str) -> String {
    let mut output = patterns::normalize_spaces(value.trim());
    for (from, to) in [
        (" ^", "^"),
        ("^ ", "^"),
        (" _", "_"),
        ("_ ", "_"),
        (" {", "{"),
        ("} ", "}"),
        ("( ", "("),
        (" )", ")"),
        ("[ ", "["),
        (" ]", "]"),
        (" ,", ","),
        (" .", "."),
        (" ;", ";"),
        (" :", ":"),
    ] {
        output = output.replace(from, to);
    }
    output
}
