use owo_colors::OwoColorize;

use crate::VERSION;

/// Print a styled banner for verbose mode
pub fn print_banner() {
    eprintln!(
        "\n{} {} {}",
        "Lectito".bold().bright_blue(),
        "v".dimmed(),
        VERSION.dimmed()
    );
    eprintln!("{}", "Extract article content from web pages\n".dimmed());
}

/// Print a styled step message
pub fn print_step(step: usize, total: usize, message: &str) {
    eprintln!("{} {}", format!("[{}/{}]", step, total).dimmed(), message.bright_cyan());
}

/// Print a success message
pub fn print_success(message: &str) {
    eprintln!("{} {}", "✓".green(), message.bright_green());
}

/// Print an info message
pub fn print_info(message: &str) {
    eprintln!("{} {}", "ℹ".blue(), message.bright_blue());
}

/// Print a warning message
pub fn print_warning(message: &str) {
    eprintln!("{} {}", "⚠".yellow(), message.bright_yellow());
}

/// Print an error message
pub fn print_error(message: &str) {
    eprintln!("{} {}", "✗".red(), message.bright_red());
}

/// Print timing information with color coding
pub fn print_timing(label: &str, duration: std::time::Duration) {
    let ms = duration.as_secs_f64() * 1000.0;
    let (color, indicator) = if ms < 50.0 {
        ("green", "fast")
    } else if ms < 100.0 {
        ("yellow", "moderate")
    } else {
        ("red", "slow")
    };

    match color {
        "green" => eprintln!(
            "  {} {:>8.2}ms ({})",
            format!("{}:", label).dimmed(),
            ms,
            indicator.dimmed()
        ),
        "yellow" => eprintln!(
            "  {} {:>8.2}ms ({})",
            format!("{}:", label).dimmed(),
            ms,
            indicator.bright_yellow()
        ),
        _ => eprintln!(
            "  {} {:>8.2}ms ({})",
            format!("{}:", label).dimmed(),
            ms,
            indicator.bright_red()
        ),
    }
}

/// Print extraction details summary
pub fn print_extraction_details(extracted: &lectito_core::ExtractedContent) {
    eprintln!("\n{}", "═".repeat(60).dimmed());
    eprintln!("{}", "Extraction Details".bold().cyan());
    eprintln!("{}", "═".repeat(60).dimmed());
    eprintln!(
        "  {} {}",
        "Top Score:".dimmed(),
        format!("{:.1}", extracted.top_score).bright_white()
    );
    eprintln!(
        "  {} {}\n",
        "Elements:".dimmed(),
        extracted.element_count.to_string().bright_white()
    );
}

/// Print timing summary
pub fn print_timing_summary(total: std::time::Duration, timings: &[(String, std::time::Duration)]) {
    eprintln!("{}", "═".repeat(60).dimmed());
    eprintln!("{}", "Timing Summary".bold().cyan());
    eprintln!("{}", "═".repeat(60).dimmed());

    for (label, duration) in timings {
        print_timing(label, *duration);
    }

    eprintln!(
        "  {} {:>8.2}ms\n",
        format!("{}:", "Total").bold().dimmed(),
        total.as_secs_f64() * 1000.0
    );
}

/// Format file size for display
pub fn format_size(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = 1024 * KB;

    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
