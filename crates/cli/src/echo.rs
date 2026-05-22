use lectito::markdown_with_toml_frontmatter;
use lectito::{Article, ExtractionDiagnostics};

use anyhow::Context;
use owo_colors::OwoColorize;

use crate::cli::{DiagnosticFormat, OutputFormat};

pub fn diagnostics(diagnostics: &ExtractionDiagnostics, format: DiagnosticFormat) -> anyhow::Result<()> {
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

pub fn parsed(
    article: Option<&Article>, format: OutputFormat, pretty: bool, source: Option<&str>,
) -> anyhow::Result<()> {
    match format {
        OutputFormat::Json => {
            if pretty {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&article).context("failed to serialize JSON")?
                );
            } else {
                println!(
                    "{}",
                    serde_json::to_string(&article).context("failed to serialize JSON")?
                );
            }
        }
        OutputFormat::Html => {
            if let Some(article) = article {
                println!("{}", article.content);
            }
        }
        OutputFormat::Markdown => {
            if let Some(article) = article {
                println!(
                    "{}",
                    markdown_with_toml_frontmatter(article, source).context("failed to serialize TOML frontmatter")?
                );
            }
        }
        OutputFormat::Text => {
            if let Some(article) = article {
                println!("{}", article.text_content);
            }
        }
    }

    Ok(())
}

pub fn readable(readable: bool, json: bool, pretty: bool) -> anyhow::Result<()> {
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
