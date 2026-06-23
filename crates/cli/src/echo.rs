use lectito::markdown_with_toml_frontmatter;
use lectito::{Article, ExtractionDiagnostics, ExtractionReport};

use anyhow::{Context, Result};
use owo_colors::OwoColorize;

use crate::cli::{DiagnosticFormat, OutputFormat};

pub struct RenderOptions<'a> {
    format: OutputFormat,
    pretty: bool,
    source: Option<&'a str>,
    frontmatter: bool,
}

impl<'a> RenderOptions<'a> {
    pub fn new(format: OutputFormat, pretty: bool, source: Option<&'a str>, frontmatter: bool) -> Self {
        Self { format, pretty, source, frontmatter }
    }
}

pub struct InspectOptions<'a> {
    source: Option<&'a str>,
    json: bool,
    pretty: bool,
}

impl<'a> InspectOptions<'a> {
    pub fn new(pretty: bool, source: Option<&'a str>, json: bool) -> Self {
        Self { pretty, source, json }
    }
}

pub fn diagnostics(diagnostics: &ExtractionDiagnostics, format: DiagnosticFormat, color: bool) -> Result<()> {
    match format {
        DiagnosticFormat::Json => {
            eprintln!(
                "{}",
                serde_json::to_string_pretty(diagnostics).context("failed to serialize diagnostics")?
            )
        }
        DiagnosticFormat::Pretty => {
            eprintln!(
                "{}",
                style("lectito diagnostics", color, |value| value.bold().blue().to_string())
            );
            eprintln!(
                "{} {:?}",
                style("outcome:", color, |value| value.bold().to_string()),
                diagnostics.outcome
            );
            if let Some(selector) = &diagnostics.content_selector {
                let status = if selector.matched {
                    style("matched", color, |value| value.green().to_string())
                } else {
                    style("not matched", color, |value| value.yellow().to_string())
                };
                eprintln!(
                    "{} {} ({status})",
                    style("content selector:", color, |value| value.bold().to_string()),
                    selector.selector
                );
            }
            if let Some(site_rule) = &diagnostics.site_rule {
                let status = if site_rule.accepted {
                    style("accepted", color, |value| value.green().to_string())
                } else {
                    style("fallback", color, |value| value.yellow().to_string())
                };
                eprintln!(
                    "{} {} {:?} text_len={} removals={} ({status})",
                    style("site rule:", color, |value| value.bold().to_string()),
                    site_rule.name,
                    site_rule.source,
                    site_rule.text_len,
                    site_rule.removals
                );
                if let Some(reason) = &site_rule.fallback_reason {
                    eprintln!(
                        "  {} {}",
                        style("reason:", color, |value| value.bold().to_string()),
                        reason
                    );
                }
            }
            for attempt in &diagnostics.attempts {
                let marker = if Some(attempt.index) == diagnostics.selected_attempt {
                    style("*", color, |value| value.green().to_string())
                } else {
                    " ".to_string()
                };
                eprintln!(
                    "{marker} {} {} {} {} {}",
                    style("attempt", color, |value| value.bold().to_string()),
                    attempt.index,
                    style("text_len=", color, |value| value.dimmed().to_string()),
                    attempt.text_len,
                    if attempt.accepted {
                        style("accepted", color, |value| value.green().to_string())
                    } else {
                        style("below threshold", color, |value| value.yellow().to_string())
                    }
                );
                if let Some(root) = &attempt.selected_root {
                    eprintln!(
                        "  {} {} (text {}, links {:.3})",
                        style("root:", color, |value| value.bold().to_string()),
                        root.selector,
                        root.text_len,
                        root.link_density
                    );
                }
                if attempt.recovery.shadow_roots_flattened > 0 || attempt.recovery.mobile_rules_applied > 0 {
                    eprintln!(
                        "  {} shadow_roots={}, mobile_rules={}",
                        style("recovery:", color, |value| value.bold().to_string()),
                        attempt.recovery.shadow_roots_flattened,
                        attempt.recovery.mobile_rules_applied
                    );
                }
                if !attempt.entry_points.is_empty() {
                    eprintln!("  {}", style("entry points:", color, |value| value.bold().to_string()));
                    for candidate in attempt.entry_points.iter().take(3) {
                        eprintln!(
                            "    {:>8.3} {} text={}",
                            candidate.score, candidate.node.selector, candidate.node.text_len
                        );
                    }
                }
                if !attempt.candidates.is_empty() {
                    eprintln!(
                        "  {}",
                        style("top candidates:", color, |value| value.bold().to_string())
                    );
                    for candidate in attempt.candidates.iter().take(5) {
                        eprintln!(
                            "    {:>8.3} {} text={} links={:.3}",
                            candidate.score,
                            candidate.node.selector,
                            candidate.node.text_len,
                            candidate.node.link_density
                        );
                    }
                }
                if let Some(cleanup) = &attempt.cleanup {
                    eprintln!(
                        "  {} text {} -> {}, elements {} -> {} (removed {})",
                        style("cleanup:", color, |value| value.bold().to_string()),
                        cleanup.text_len_before,
                        cleanup.text_len_after,
                        cleanup.element_count_before,
                        cleanup.element_count_after,
                        cleanup.removed_elements
                    );
                }
            }
        }
    }

    Ok(())
}

pub fn render_article(article: Option<&Article>, opts: RenderOptions) -> Result<String> {
    match opts.format {
        OutputFormat::Json => match opts.pretty {
            true => serde_json::to_string_pretty(&article).context("failed to serialize JSON"),
            false => serde_json::to_string(&article).context("failed to serialize JSON"),
        },
        OutputFormat::Html => match article {
            Some(article) => Ok(article.content.clone()),
            None => Ok(String::new()),
        },
        OutputFormat::Markdown => match article {
            Some(article) => match opts.frontmatter {
                true => {
                    markdown_with_toml_frontmatter(article, opts.source).context("failed to serialize TOML frontmatter")
                }
                false => Ok(article.markdown.clone()),
            },
            None => Ok(String::new()),
        },
        OutputFormat::Text => match article {
            Some(article) => Ok(article.text_content.clone()),
            None => Ok(String::new()),
        },
    }
}

pub fn readable(readable: bool, json: bool, pretty: bool) -> Result<()> {
    if json {
        let value = serde_json::json!({ "readable": readable });
        if pretty {
            println!(
                "{}",
                serde_json::to_string_pretty(&value).context("failed to serialize JSON")?
            );
        } else {
            println!("{}", serde_json::to_string(&value).context("failed to serialize JSON")?);
        }
    } else {
        println!("{readable}");
    }

    Ok(())
}

pub fn inspect(report: &ExtractionReport, opts: InspectOptions) -> Result<String> {
    if opts.json {
        let value = serde_json::json!({
            "source": opts.source,
            "article": report.article,
            "diagnostics": report.diagnostics,
        });
        if opts.pretty {
            return serde_json::to_string_pretty(&value).context("failed to serialize inspect JSON");
        }
        return serde_json::to_string(&value).context("failed to serialize inspect JSON");
    }

    let mut lines: Vec<String> = vec!["lectito inspect".to_string()];
    if let Some(source) = opts.source {
        lines.push(format!("source: {source}"));
    }
    lines.push(format!("outcome: {:?}", report.diagnostics.outcome));

    match &report.article {
        Some(article) => {
            if let Some(title) = &article.title {
                lines.push(format!("title: {title}"));
            }
            if let Some(byline) = &article.byline {
                lines.push(format!("byline: {byline}"));
            }
            if let Some(site_name) = &article.site_name {
                lines.push(format!("site: {site_name}"));
            }
            if let Some(published_time) = &article.published_time {
                lines.push(format!("published: {published_time}"));
            }
            lines.push(format!("text chars: {}", article.text_content.chars().count()));
            lines.push(format!("content html bytes: {}", article.content.len()));
        }
        None => lines.push("article: none".to_string()),
    }

    lines.push(format!("attempts: {}", report.diagnostics.attempts.len()));
    if let Some(index) = report.diagnostics.selected_attempt {
        lines.push(format!("selected attempt: {index}"));
    }
    if let Some(selector) = &report.diagnostics.content_selector {
        lines.push(format!(
            "content selector: {} ({})",
            selector.selector,
            if selector.matched { "matched" } else { "not matched" }
        ));
    }
    if let Some(site_rule) = &report.diagnostics.site_rule {
        lines.push(format!(
            "site rule: {} {:?} ({})",
            site_rule.name,
            site_rule.source,
            if site_rule.accepted { "accepted" } else { "fallback" }
        ));
    }
    if let Some(attempt) = report.diagnostics.selected_attempt.and_then(|index| {
        report
            .diagnostics
            .attempts
            .iter()
            .find(|attempt| attempt.index == index)
    }) {
        if let Some(root) = &attempt.selected_root {
            lines.push(format!(
                "root: {} text={} links={:.3}",
                root.selector, root.text_len, root.link_density
            ));
        }
        lines.push(format!(
            "cleanup: text {} -> {}, elements {} -> {}",
            attempt
                .cleanup
                .as_ref()
                .map(|cleanup| cleanup.text_len_before)
                .unwrap_or(0),
            attempt
                .cleanup
                .as_ref()
                .map(|cleanup| cleanup.text_len_after)
                .unwrap_or(0),
            attempt
                .cleanup
                .as_ref()
                .map(|cleanup| cleanup.element_count_before)
                .unwrap_or(0),
            attempt
                .cleanup
                .as_ref()
                .map(|cleanup| cleanup.element_count_after)
                .unwrap_or(0)
        ));
    }

    Ok(lines.join("\n"))
}

fn style(value: &str, color: bool, apply: impl FnOnce(&str) -> String) -> String {
    if color { apply(value) } else { value.to_string() }
}
