use lectito::{ReadabilityOptions, extract};

const HTML: &str = r#"
<!doctype html>
<html lang="en">
  <head>
    <title>Readable HTML in Rust - Example Site</title>
    <meta name="author" content="Lectito Team">
  </head>
  <body>
    <nav>Home | Archive | About</nav>
    <article>
      <h1>Readable HTML in Rust</h1>
      <p>
        Lectito extracts the article body and removes surrounding page chrome.
        Callers pass HTML from a crawler, browser, cache, or test fixture.
      </p>
      <p>
        The result includes cleaned HTML, Markdown, plain text, metadata, and
        optional diagnostics for tuning extraction.
      </p>
      <p>
        Relative links such as <a href="/docs">the documentation</a> are
        resolved with the base URL supplied by the caller.
      </p>
    </article>
    <aside>Related links and newsletter signup</aside>
  </body>
</html>
"#;

fn main() -> Result<(), lectito::Error> {
    let options = ReadabilityOptions { char_threshold: 0, ..Default::default() };
    let article =
        extract(HTML, Some("https://example.com/post"), &options)?.expect("example article should be readable");

    println!("# {}", article.title.unwrap_or_else(|| "Untitled".to_string()));
    println!();
    println!("{}", article.markdown.trim());

    Ok(())
}
