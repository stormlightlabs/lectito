use lectito::markdown_with_toml_frontmatter;
use lectito::{Article, ExtractionDiagnostics, ExtractionReport};

use anyhow::{Context, Result};
use owo_colors::OwoColorize;

use crate::cli::{DiagnosticFormat, OutputFormat};

pub fn diagnostics(diagnostics: &ExtractionDiagnostics, format: DiagnosticFormat) -> Result<()> {
    match format {
        DiagnosticFormat::Json => {
            eprintln!(
                "{}",
                serde_json::to_string_pretty(diagnostics).context("failed to serialize diagnostics")?
            );
        }
        DiagnosticFormat::Pretty => {
            eprintln!("{}", "lectito diagnostics".bold().blue());
            eprintln!("{} {:?}", "outcome:".bold(), diagnostics.outcome);
            if let Some(selector) = &diagnostics.content_selector {
                let status =
                    if selector.matched { "matched".green().to_string() } else { "not matched".yellow().to_string() };
                eprintln!("{} {} ({status})", "content selector:".bold(), selector.selector);
            }
            if let Some(site_rule) = &diagnostics.site_rule {
                let status =
                    if site_rule.accepted { "accepted".green().to_string() } else { "fallback".yellow().to_string() };
                eprintln!(
                    "{} {} {:?} text_len={} removals={} ({status})",
                    "site rule:".bold(),
                    site_rule.name,
                    site_rule.source,
                    site_rule.text_len,
                    site_rule.removals
                );
                if let Some(reason) = &site_rule.fallback_reason {
                    eprintln!("  {} {}", "reason:".bold(), reason);
                }
            }
            for attempt in &diagnostics.attempts {
                let marker = if Some(attempt.index) == diagnostics.selected_attempt {
                    "*".green().to_string()
                } else {
                    " ".to_string()
                };
                eprintln!(
                    "{marker} {} {} {} {} {}",
                    "attempt".bold(),
                    attempt.index,
                    "text_len=".dimmed(),
                    attempt.text_len,
                    if attempt.accepted {
                        "accepted".green().to_string()
                    } else {
                        "below threshold".yellow().to_string()
                    }
                );
                if let Some(root) = &attempt.selected_root {
                    eprintln!(
                        "  {} {} (text {}, links {:.3})",
                        "root:".bold(),
                        root.selector,
                        root.text_len,
                        root.link_density
                    );
                }
                if attempt.recovery.shadow_roots_flattened > 0 || attempt.recovery.mobile_rules_applied > 0 {
                    eprintln!(
                        "  {} shadow_roots={}, mobile_rules={}",
                        "recovery:".bold(),
                        attempt.recovery.shadow_roots_flattened,
                        attempt.recovery.mobile_rules_applied
                    );
                }
                if !attempt.entry_points.is_empty() {
                    eprintln!("  {}", "entry points:".bold());
                    for candidate in attempt.entry_points.iter().take(3) {
                        eprintln!(
                            "    {:>8.3} {} text={}",
                            candidate.score, candidate.node.selector, candidate.node.text_len
                        );
                    }
                }
                if !attempt.candidates.is_empty() {
                    eprintln!("  {}", "top candidates:".bold());
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
                        "cleanup:".bold(),
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

pub fn render_article(
    article: Option<&Article>, format: OutputFormat, pretty: bool, source: Option<&str>, frontmatter: bool,
) -> Result<String> {
    match format {
        OutputFormat::Json => {
            if pretty {
                serde_json::to_string_pretty(&article).context("failed to serialize JSON")
            } else {
                serde_json::to_string(&article).context("failed to serialize JSON")
            }
        }
        OutputFormat::Html => {
            if let Some(article) = article {
                Ok(article.content.clone())
            } else {
                Ok(String::new())
            }
        }
        OutputFormat::Markdown => {
            if let Some(article) = article {
                if frontmatter {
                    markdown_with_toml_frontmatter(article, source).context("failed to serialize TOML frontmatter")
                } else {
                    Ok(article.markdown.clone())
                }
            } else {
                Ok(String::new())
            }
        }
        OutputFormat::Text => {
            if let Some(article) = article {
                Ok(article.text_content.clone())
            } else {
                Ok(String::new())
            }
        }
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

pub fn inspect(report: &ExtractionReport, source: Option<&str>, json: bool, pretty: bool) -> Result<String> {
    if json {
        let value = serde_json::json!({
            "source": source,
            "article": report.article,
            "diagnostics": report.diagnostics,
        });
        if pretty {
            return serde_json::to_string_pretty(&value).context("failed to serialize inspect JSON");
        }
        return serde_json::to_string(&value).context("failed to serialize inspect JSON");
    }

    let mut lines = Vec::new();
    lines.push("lectito inspect".to_string());
    if let Some(source) = source {
        lines.push(format!("source: {source}"));
    }
    lines.push(format!("outcome: {:?}", report.diagnostics.outcome));

    if let Some(article) = &report.article {
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
    } else {
        lines.push("article: none".to_string());
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
