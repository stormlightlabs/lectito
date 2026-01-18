use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=OUT_DIR");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let completions_dir = out_dir.join("completions");

    fs::create_dir_all(&completions_dir).unwrap();

    let mut cmd = clap::Command::new("lectito")
        .version("1.0.0")
        .author("Lectito Contributors")
        .about("Extract article content from web pages")
        .arg(clap::arg!(<INPUT> "URL to fetch, local HTML file, or '-' for stdin"))
        .arg(
            clap::arg!(-o --output <FILE> "Output file (default: stdout)")
                .value_name("FILE")
                .value_parser(clap::value_parser!(std::path::PathBuf)),
        )
        .arg(
            clap::arg!(-f --format <FORMAT> "Output format (markdown, html, text, json)")
                .value_name("FORMAT")
                .default_value("markdown")
                .value_parser(["markdown", "html", "text", "json"]),
        )
        .arg(clap::arg!(--references "Include reference table with all links (Markdown/JSON only)"))
        .arg(clap::arg!(--frontmatter "Include TOML frontmatter (Markdown only)"))
        .arg(clap::arg!(--timeout <SECS> "HTTP timeout in seconds").default_value("30"))
        .arg(clap::arg!(--user_agent <UA> "Custom User-Agent for HTTP requests").value_name("UA"))
        .arg(
            clap::arg!(--config_dir <DIR> "Custom site config directory")
                .value_name("DIR")
                .value_parser(clap::value_parser!(std::path::PathBuf)),
        )
        .arg(
            clap::arg!(--char_threshold <NUM> "Minimum character threshold for content candidates")
                .default_value("500"),
        )
        .arg(clap::arg!(--max_elements <NUM> "Maximum number of top candidates to track").default_value("5"))
        .arg(clap::arg!(--no_images "Strip images from output"))
        .arg(clap::arg!(-v --verbose "Enable debug logging"))
        .arg(
            clap::arg!(--completions <SHELL> "Generate shell completion script")
                .value_name("SHELL")
                .value_parser(["bash", "zsh", "fish", "powershell"]),
        );

    clap_complete::generate_to(clap_complete::shells::Bash, &mut cmd, "lectito", &completions_dir).unwrap();
    clap_complete::generate_to(clap_complete::shells::Zsh, &mut cmd, "lectito", &completions_dir).unwrap();
    clap_complete::generate_to(clap_complete::shells::Fish, &mut cmd, "lectito", &completions_dir).unwrap();
    clap_complete::generate_to(clap_complete::shells::PowerShell, &mut cmd, "lectito", &completions_dir).unwrap();

    println!(
        "cargo:warning=Shell completions generated in: {}",
        completions_dir.display()
    );
}
