use anyhow::Context;
use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, CACHE_CONTROL, HeaderMap, HeaderValue, LOCATION, REFERER};
use reqwest::redirect::Policy;
use reqwest::{StatusCode, Url, blocking::Client};
use scraper::{Html, Selector};
use std::io::{self, Read};
use std::path::Path;
use std::process;
use std::time::Duration;

pub const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36";
pub const CURL_USER_AGENT: &str = "curl/8.7.1";
pub const MAX_REDIRECTS: usize = 10;
pub const FETCH_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Clone, Copy)]
enum FetchProfile {
    Browser,
    Curl,
}

impl FetchProfile {
    fn user_agent(self) -> &'static str {
        match self {
            Self::Browser => USER_AGENT,
            Self::Curl => CURL_USER_AGENT,
        }
    }

    fn headers(self) -> HeaderMap {
        match self {
            Self::Browser => {
                let mut headers = HeaderMap::new();
                headers.insert(
                    ACCEPT,
                    HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"),
                );
                headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));
                headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
                headers.insert(REFERER, HeaderValue::from_static("https://www.google.com/"));
                headers
            }
            Self::Curl => {
                let mut headers = HeaderMap::new();
                headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
                headers
            }
        }
    }
}

pub struct InputDocument {
    html: String,
    base_url: Option<String>,
}

impl InputDocument {
    pub fn html(&self) -> &str {
        &self.html
    }

    pub fn base_url(&self) -> Option<&str> {
        self.base_url.as_deref()
    }

    pub fn read_source(input: Option<&str>, read_stdin: bool, base_url: Option<&str>) -> anyhow::Result<InputDocument> {
        if read_stdin && input.is_some_and(|value| value != "-") {
            anyhow::bail!("cannot combine --stdin with an input path or URL");
        }

        if read_stdin || input == Some("-") {
            let mut html = String::new();
            io::stdin().read_to_string(&mut html).context("failed to read stdin")?;
            return Ok(InputDocument { html, base_url: base_url.map(str::to_string) });
        }

        let Some(input) = input else {
            anyhow::bail!("pass a URL, a file path, or '-' for stdin");
        };

        if input.starts_with("http://") || input.starts_with("https://") {
            if base_url.is_some() {
                anyhow::bail!("cannot combine --base-url with a URL input");
            }
            return Self::read(None, false, Some(input));
        }

        let path = Path::new(input);
        let html = std::fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
        Ok(InputDocument { html, base_url: base_url.map(str::to_string) })
    }

    pub fn read(path: Option<&Path>, read_stdin: bool, url: Option<&str>) -> anyhow::Result<InputDocument> {
        if read_stdin && path.is_some() {
            anyhow::bail!("cannot combine --stdin with a file path");
        }

        if read_stdin {
            let mut html = String::new();
            io::stdin().read_to_string(&mut html).context("failed to read stdin")?;
            return Ok(InputDocument { html, base_url: url.map(str::to_string) });
        }

        if let Some(path) = path {
            let html = std::fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
            return Ok(InputDocument { html, base_url: url.map(str::to_string) });
        }

        if let Some(url) = url {
            return Self::profile(url, FetchProfile::Browser).or_else(|error| {
                let message = format!("{error:?}");

                if message.contains("403 Forbidden") || message.contains("429 Too Many Requests") {
                    Self::profile(url, FetchProfile::Curl).or_else(|err| {
                        let curl_err_message = format!("{err:?}");
                        if curl_err_message.contains("403 Forbidden")
                            || curl_err_message.contains("429 Too Many Requests")
                        {
                            Self::curl(url)
                        } else {
                            Err(err)
                        }
                    })
                } else {
                    Err(error)
                }
            });
        }

        anyhow::bail!("pass either --stdin, a file path, or --url without a file path")
    }

    fn profile(url: &str, profile: FetchProfile) -> anyhow::Result<InputDocument> {
        let client = Client::builder()
            .user_agent(profile.user_agent())
            .default_headers(profile.headers())
            .redirect(Policy::none())
            .timeout(FETCH_TIMEOUT)
            .build()
            .with_context(|| format!("failed to build HTTP client for {url}"))?;

        let mut current_url = Url::parse(url).with_context(|| format!("invalid URL: {url}"))?;

        for redirect_count in 0..=MAX_REDIRECTS {
            let response = client
                .get(current_url.clone())
                .send()
                .with_context(|| format!("HTTP request failed for {current_url}"))?;

            if matches!(
                response.status(),
                StatusCode::MOVED_PERMANENTLY
                    | StatusCode::FOUND
                    | StatusCode::SEE_OTHER
                    | StatusCode::TEMPORARY_REDIRECT
                    | StatusCode::PERMANENT_REDIRECT
            ) {
                let location = response
                    .headers()
                    .get(LOCATION)
                    .ok_or_else(|| anyhow::anyhow!("redirect response missing Location header for {current_url}"))?
                    .to_str()
                    .with_context(|| format!("redirect Location header is not valid UTF-8 for {current_url}"))?;

                if redirect_count == MAX_REDIRECTS {
                    anyhow::bail!("too many redirects while fetching {url}");
                }

                current_url = current_url
                    .join(location)
                    .with_context(|| format!("failed to resolve redirect from {current_url} to {location}"))?;
                continue;
            }

            let response = response
                .error_for_status()
                .with_context(|| format!("HTTP request failed for {current_url}"))?;
            let html = response
                .text()
                .with_context(|| format!("failed to read response body for {current_url}"))?;

            if let Some(redirect_url) = html_redirect_target(&html, &current_url) {
                if redirect_count == MAX_REDIRECTS {
                    anyhow::bail!("too many redirects while fetching {url}");
                }

                current_url = redirect_url;
                continue;
            }

            return Ok(InputDocument { html, base_url: Some(current_url.to_string()) });
        }

        unreachable!("redirect loop exits by returning a response or bailing at the redirect limit")
    }

    fn curl(url: &str) -> anyhow::Result<InputDocument> {
        let marker = "\nLECTITO_EFFECTIVE_URL:";
        let output = process::Command::new("curl")
            .args([
                "-sS",
                "-L",
                "--fail",
                "--compressed",
                "--max-time",
                "20",
                "-A",
                CURL_USER_AGENT,
                "-w",
                &format!("{marker}%{{url_effective}}"),
                url,
            ])
            .output()
            .with_context(|| format!("failed to run curl fallback for {url}"))?;

        if !output.status.success() {
            anyhow::bail!("curl fallback failed for {url} with status {}", output.status);
        }

        let output = String::from_utf8(output.stdout).context("curl fallback returned non-UTF-8 body")?;
        let Some((html, effective_url)) = output.rsplit_once(marker) else {
            anyhow::bail!("curl fallback output did not include final URL for {url}");
        };

        Ok(InputDocument { html: html.to_string(), base_url: Some(effective_url.trim().to_string()) })
    }
}

pub fn html_redirect_target(html: &str, current_url: &Url) -> Option<Url> {
    if html.len() > 4096 {
        return None;
    }

    let document = Html::parse_document(html);
    let title_selector = Selector::parse("title").expect("valid title selector");
    let title_is_redirect = document
        .select(&title_selector)
        .next()
        .map(|title| title.text().collect::<String>().trim().eq_ignore_ascii_case("redirect"))
        .unwrap_or(false);
    let text = document.root_element().text().collect::<String>().to_lowercase();

    if !(title_is_redirect || text.contains("redirected") || text.contains("redirecting")) {
        return None;
    }

    let selector = Selector::parse("meta[http-equiv]").expect("valid meta refresh selector");
    document
        .select(&selector)
        .find_map(|node| {
            let value = node.value();
            let http_equiv = value.attr("http-equiv")?;
            if !http_equiv.eq_ignore_ascii_case("refresh") {
                return None;
            }

            value.attr("content")?.split(';').find_map(|part| {
                let part = part.trim();
                let (name, value) = part.split_once('=')?;
                if name.trim().eq_ignore_ascii_case("url") {
                    Some(value.trim().trim_matches(['"', '\'']).to_string())
                } else {
                    None
                }
            })
        })
        .or_else(|| {
            let selector = Selector::parse("a[href]").expect("valid link selector");
            document
                .select(&selector)
                .find_map(|node| node.value().attr("href").map(str::to_string))
        })
        .and_then(|target| current_url.join(&target).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_small_meta_refresh_redirect_page() {
        let html = r#"<!doctype html>
<meta charset="utf-8">
<title>Redirect</title>
<noscript>
  <meta http-equiv="refresh" content="0; url=/2024/11/28/Rust-1.83.0/">
</noscript>
<p><a href="/2024/11/28/Rust-1.83.0/">Click here</a> to be redirected.</p>"#;
        let current_url = Url::parse("https://blog.rust-lang.org/2024/11/28/Rust-1.83.0.html").unwrap();

        let target = html_redirect_target(html, &current_url).unwrap();

        assert_eq!(target.as_str(), "https://blog.rust-lang.org/2024/11/28/Rust-1.83.0/");
    }

    #[test]
    fn ignores_regular_small_article_html() {
        let html = r#"<!doctype html><title>Article</title><article><p>This is article content.</p></article>"#;
        let current_url = Url::parse("https://example.com/post").unwrap();

        assert!(html_redirect_target(html, &current_url).is_none());
    }
}
