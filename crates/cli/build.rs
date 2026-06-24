use clap::CommandFactory;
use std::io::Error;
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    process::Command,
};

#[path = "src/cli.rs"]
mod cli;

fn main() {
    let workspace = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set"))
        .parent()
        .and_then(Path::parent)
        .expect("crate lives under workspace/crates")
        .to_path_buf();
    let docs_path = workspace.join("scripts").join("doc").join("cli.txt");
    let manual = render_manual(cli::Cli::command()).expect("render CLI manual");
    write_if_changed(&docs_path, manual).expect("write CLI manual");

    println!("cargo:rerun-if-changed=src/cli.rs");
    println!("cargo:rerun-if-changed=build.rs");
}

fn render_manual(command: clap::Command) -> std::io::Result<String> {
    let mut rendered = String::new();
    render_command(command, vec!["lectito".to_string()], &mut rendered)?;
    Ok(rendered)
}

fn render_command(command: clap::Command, path: Vec<String>, rendered: &mut String) -> std::io::Result<()> {
    let subcommands = command.get_subcommands().cloned().collect::<Vec<_>>();
    let display_name = path.join(" ");
    let page_name = path.join("-");
    let mut command = command;
    command = command
        .name(leak_string(page_name.clone()))
        .bin_name(leak_string(display_name));

    let mut roff = Vec::new();
    clap_mangen::Man::new(command).render(&mut roff)?;
    let text = roff_to_text(&page_name, &roff)?;

    if !rendered.is_empty() {
        rendered.push_str("\n\n");
    }
    rendered.push_str(text.trim());
    rendered.push('\n');

    for subcommand in subcommands {
        let mut subcommand_path = path.clone();
        subcommand_path.push(subcommand.get_name().to_string());
        render_command(subcommand, subcommand_path, rendered)?;
    }

    Ok(())
}

fn roff_to_text(page_name: &str, roff: &[u8]) -> std::io::Result<String> {
    let roff_path = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR is set")).join(format!("{page_name}.1"));
    std::fs::write(&roff_path, roff)?;

    let roff_path_text = roff_path.to_string_lossy();
    let escaped_path = shell_escape::escape(Cow::Borrowed(roff_path_text.as_ref()));
    let script = format!(
        "if command -v mandoc >/dev/null 2>&1 && command -v col >/dev/null 2>&1; then mandoc -T utf8 {0} | col -b; \
         elif command -v mandoc >/dev/null 2>&1; then mandoc -T utf8 {0}; \
         elif command -v groff >/dev/null 2>&1 && command -v col >/dev/null 2>&1; then groff -T utf8 -man {0} | col -b; \
         else cat {0}; fi",
        escaped_path
    );
    let output = Command::new("sh").arg("-c").arg(script).output()?;

    if !output.status.success() {
        return Err(Error::other(String::from_utf8_lossy(&output.stderr).into_owned()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn write_if_changed(path: &Path, contents: String) -> std::io::Result<()> {
    if std::fs::read_to_string(path).is_ok_and(|current| current == contents) {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, contents)
}

fn leak_string(value: String) -> &'static str {
    Box::leak(value.into_boxed_str())
}
