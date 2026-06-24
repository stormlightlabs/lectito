use anyhow::Context;
use reqwest::header::{
    ACCEPT, ACCEPT_LANGUAGE, CACHE_CONTROL, CONTENT_TYPE, HeaderMap, HeaderValue, LAST_MODIFIED, LOCATION, REFERER,
};
use reqwest::redirect::Policy;
use reqwest::{StatusCode, Url, blocking::Client};
use scraper::{Html, Selector};
use std::io::{self, Read};
use std::path::Path;
use std::process;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::atproto::{self, AtprotoClient};

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
    content_type: Option<String>,
    last_modified: Option<String>,
}

impl InputDocument {
    fn new(html: String, base_url: Option<String>, content_type: Option<String>, lastmod: Option<String>) -> Self {
        Self { html, base_url, content_type, last_modified: lastmod }
    }

    pub fn html(&self) -> &str {
        &self.html
    }

    pub fn base_url(&self) -> Option<&str> {
        self.base_url.as_deref()
    }

    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }

    pub fn last_modified(&self) -> Option<&str> {
        self.last_modified.as_deref()
    }

    pub fn read_src(input: Option<&str>, read_stdin: bool, base_url: Option<&str>) -> anyhow::Result<InputDocument> {
        if read_stdin && input.is_some_and(|value| value != "-") {
            anyhow::bail!("cannot combine --stdin with an input path or URL");
        }

        if read_stdin || input == Some("-") {
            let mut html = String::new();
            io::stdin().read_to_string(&mut html).context("failed to read stdin")?;
            return Ok(InputDocument::new(html, base_url.map(str::to_string), None, None));
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

        if input.starts_with("at://") {
            if base_url.is_some() {
                anyhow::bail!("cannot combine --base-url with an AT URI input");
            }
            return Self::atproto(input);
        }

        let path = Path::new(input);
        let html = std::fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
        Ok(InputDocument::new(html, base_url.map(str::to_string), None, None))
    }

    pub fn read(path: Option<&Path>, read_stdin: bool, url: Option<&str>) -> anyhow::Result<InputDocument> {
        if read_stdin && path.is_some() {
            anyhow::bail!("cannot combine --stdin with a file path");
        }

        if read_stdin {
            let mut html = String::new();
            io::stdin().read_to_string(&mut html).context("failed to read stdin")?;
            return Ok(InputDocument::new(html, url.map(str::to_string), None, None));
        }

        if let Some(path) = path {
            let html = std::fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
            return Ok(InputDocument::new(html, url.map(str::to_string), None, None));
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
            let content_type = response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .map(str::to_string);
            let last_modified = response
                .headers()
                .get(LAST_MODIFIED)
                .and_then(|value| value.to_str().ok())
                .map(str::to_string);
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

            let html = standard_site_html(&client, &html, Some(current_url.as_str())).unwrap_or(html);

            return Ok(InputDocument::new(
                html,
                Some(current_url.to_string()),
                content_type,
                last_modified,
            ));
        }

        unreachable!("redirect loop exits by returning a response or bailing at the redirect limit")
    }

    fn curl(url: &str) -> anyhow::Result<InputDocument> {
        let marker = "\nLECTITO_EFFECTIVE_URL:";
        let headers_path = {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default();
            std::env::temp_dir().join(format!("lectito-curl-headers-{}-{nanos}", std::process::id()))
        };
        let output = process::Command::new("curl")
            .args(["-sS", "-L", "--fail", "--compressed", "--max-time", "20", "-D"])
            .arg(&headers_path)
            .args([
                "-A",
                CURL_USER_AGENT,
                "-w",
                &format!("{marker}%{{url_effective}}\nLECTITO_CONTENT_TYPE:%{{content_type}}"),
                url,
            ])
            .output()
            .with_context(|| format!("failed to run curl fallback for {url}"))?;

        if !output.status.success() {
            let _ = std::fs::remove_file(&headers_path);
            anyhow::bail!("curl fallback failed for {url} with status {}", output.status);
        }

        let last_modified = std::fs::read_to_string(&headers_path)
            .ok()
            .and_then(|headers| final_header_value(&headers, "last-modified"));
        let _ = std::fs::remove_file(&headers_path);
        let output = String::from_utf8(output.stdout).context("curl fallback returned non-UTF-8 body")?;
        let Some((html, metadata)) = output.rsplit_once(marker) else {
            anyhow::bail!("curl fallback output did not include final URL for {url}");
        };
        let (effective_url, content_type) = metadata
            .split_once("\nLECTITO_CONTENT_TYPE:")
            .map(|(url, content_type)| {
                let val = content_type.trim();
                let value = (!val.is_empty()).then(|| val.to_string());
                (url.trim(), value)
            })
            .unwrap_or((metadata.trim(), None));

        Ok(InputDocument::new(
            html.to_string(),
            Some(effective_url.to_string()),
            content_type,
            last_modified,
        ))
    }

    fn atproto(at_uri: &str) -> anyhow::Result<InputDocument> {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .timeout(FETCH_TIMEOUT)
            .build()
            .with_context(|| format!("failed to build ATProto client for {at_uri}"))?;
        let atproto = AtprotoClient::new(client);
        let record = atproto
            .get_record(at_uri)
            .with_context(|| format!("failed to resolve AT URI {at_uri}"))?;
        let metadata = atproto.standard_site_render_metadata(&record, None).unwrap_or_default();
        let html = atproto::standard_site_document_html(&record, None, &metadata)?.ok_or_else(|| {
            anyhow::anyhow!("AT URI {at_uri} did not resolve to renderable Standard.site document content")
        })?;

        Ok(InputDocument::new(html, None, Some("text/html".to_string()), None))
    }
}

fn standard_site_html(client: &Client, html: &str, source_url: Option<&str>) -> Option<String> {
    let at_uri = atproto::standard_site_link(html)?;
    let atproto = AtprotoClient::new(client.clone());
    let record = atproto.get_record(&at_uri).ok()?;
    let metadata = atproto
        .standard_site_render_metadata(&record, source_url)
        .unwrap_or_default();
    atproto::standard_site_document_html(&record, source_url, &metadata)
        .ok()
        .flatten()
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

fn final_header_value(headers: &str, name: &str) -> Option<String> {
    let mut current = None;

    for line in headers.lines() {
        let line = line.trim_end_matches('\r');
        if line.starts_with("HTTP/") {
            current = None;
            continue;
        }
        let Some((header_name, value)) = line.split_once(':') else {
            continue;
        };
        if header_name.trim().eq_ignore_ascii_case(name) {
            let value = value.trim();
            if !value.is_empty() {
                current = Some(value.to_string());
            }
        }
    }

    current
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpListener;
    use std::thread;

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

    #[test]
    fn captures_http_content_type() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let address = listener.local_addr().expect("test server address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept test request");
            let body = "# Hello\n";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/markdown; charset=utf-8\r\nLast-Modified: Wed, 01 May 2024 10:00:00 GMT\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).expect("write response");
        });

        let url = format!("http://{address}/doc.md");
        let document = InputDocument::read_src(Some(&url), false, None).expect("read test URL");

        server.join().expect("join test server");
        assert_eq!(document.content_type(), Some("text/markdown; charset=utf-8"));
        assert_eq!(document.last_modified(), Some("Wed, 01 May 2024 10:00:00 GMT"));
        assert_eq!(document.html(), "# Hello\n");
    }

    #[test]
    fn final_header_value_uses_final_response_block() {
        let headers = "\
HTTP/1.1 301 Moved Permanently\r\n\
Last-Modified: Tue, 30 Apr 2024 10:00:00 GMT\r\n\
Location: /final\r\n\
\r\n\
HTTP/1.1 200 OK\r\n\
Content-Type: text/html\r\n\
Last-Modified: Wed, 01 May 2024 10:00:00 GMT\r\n\
\r\n";

        assert_eq!(
            final_header_value(headers, "Last-Modified").as_deref(),
            Some("Wed, 01 May 2024 10:00:00 GMT")
        );
    }

    #[test]
    fn final_header_value_ignores_redirect_header_when_final_response_lacks_it() {
        let headers = "\
HTTP/1.1 301 Moved Permanently\r\n\
Last-Modified: Tue, 30 Apr 2024 10:00:00 GMT\r\n\
Location: /final\r\n\
\r\n\
HTTP/1.1 200 OK\r\n\
Content-Type: text/html\r\n\
\r\n";

        assert_eq!(final_header_value(headers, "Last-Modified"), None);
    }
}
