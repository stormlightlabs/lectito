use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};

struct TestServer {
    addr: SocketAddr,
}

impl TestServer {
    fn new(handler: impl Fn(String) -> TestResponse + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let addr = listener.local_addr().expect("local addr");
        let handler = Arc::new(handler);

        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let handler = Arc::clone(&handler);
                thread::spawn(move || handle_connection(stream, handler));
            }
        });

        Self { addr }
    }

    fn url(&self, path: &str) -> String {
        format!("http://{}{}", self.addr, path)
    }
}

#[derive(Clone)]
struct TestResponse {
    status: u16,
    content_type: &'static str,
    body: String,
    location: Option<String>,
    delay: Duration,
}

impl TestResponse {
    fn html(body: String) -> Self {
        Self::status(200, "text/html; charset=utf-8", body)
    }

    fn redirect(location: &str) -> Self {
        Self {
            status: 302,
            content_type: "text/plain",
            body: String::new(),
            location: Some(location.to_string()),
            delay: Duration::ZERO,
        }
    }

    fn status(status: u16, content_type: &'static str, body: impl Into<String>) -> Self {
        Self { status, content_type, body: body.into(), location: None, delay: Duration::ZERO }
    }

    fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
}

fn run_mcp(lines: &[String], envs: &[(&str, &str)]) -> Vec<Value> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_lectito-mcp"));
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in envs {
        command.env(key, value);
    }

    let mut child = command.spawn().expect("spawn lectito-mcp");
    {
        let stdin = child.stdin.as_mut().expect("child stdin");
        for line in lines {
            writeln!(stdin, "{line}").expect("write request");
        }
    }

    let output = child.wait_with_output().expect("wait for lectito-mcp");
    assert!(
        output.status.success(),
        "server exited with {}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout)
        .expect("stdout is utf8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("response is json"))
        .collect()
}

fn request(id: impl Into<Value>, method: &str, params: Value) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id.into(),
        "method": method,
        "params": params,
    })
    .to_string()
}

fn tool_call(id: impl Into<Value>, name: &str, arguments: Value) -> String {
    request(id, "tools/call", json!({ "name": name, "arguments": arguments }))
}

fn article(response: &Value) -> &Value {
    &response["result"]["structuredContent"]["article"]
}

fn assert_tool_error_contains(response: &Value, expected: &str) {
    assert_eq!(response["result"]["isError"], true);
    let error = response["result"]["structuredContent"]["error"]
        .as_str()
        .expect("tool error");
    assert!(error.contains(expected), "{error}");
}

fn article_html() -> String {
    let paragraph = "This fixture paragraph has enough article-like text for \
        the default readability threshold. ";
    format!(
        r#"<!doctype html>
<html>
  <head><title>Fixture Article</title></head>
  <body>
    <nav>Navigation chrome</nav>
    <article>
      <h1>Fixture Article</h1>
      <p>{}</p>
      <p>{}</p>
      <p>{}</p>
      <p>{}</p>
      <p>{}</p>
      <p>{}</p>
      <p>{}</p>
      <p>{}</p>
    </article>
  </body>
</html>"#,
        paragraph, paragraph, paragraph, paragraph, paragraph, paragraph, paragraph, paragraph
    )
}

fn handle_connection(stream: TcpStream, handler: Arc<dyn Fn(String) -> TestResponse + Send + Sync>) {
    let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() {
        return;
    }

    let mut header = String::new();
    while reader.read_line(&mut header).is_ok() && header != "\r\n" {
        header.clear();
    }

    let path = request_line
        .split_whitespace()
        .nth(1)
        .and_then(|target| target.split('?').next())
        .unwrap_or("/")
        .to_string();
    let response = handler(path);
    if !response.delay.is_zero() {
        thread::sleep(response.delay);
    }
    write_response(stream, response);
}

fn write_response(mut stream: TcpStream, response: TestResponse) {
    let reason = match response.status {
        200 => "OK",
        302 => "Found",
        404 => "Not Found",
        _ => "Error",
    };
    let mut headers = HashMap::from([
        ("Content-Type", response.content_type.to_string()),
        ("Content-Length", response.body.len().to_string()),
        ("Connection", "close".to_string()),
    ]);
    if let Some(location) = response.location {
        headers.insert("Location", location);
    }

    let _ = write!(stream, "HTTP/1.1 {} {}\r\n", response.status, reason);
    for (name, value) in headers {
        let _ = write!(stream, "{name}: {value}\r\n");
    }
    let _ = write!(stream, "\r\n{}", response.body);
    let _ = stream.flush();

    let mut drain = [0; 16];
    let _ = stream.read(&mut drain);
}

#[test]
fn stdio_handles_protocol_requests_and_errors() {
    let lines = vec![
        request(1, "initialize", json!({})),
        json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }).to_string(),
        request(2, "tools/list", json!({})),
        "{not-json".to_string(),
        request(3, "missing/method", json!({})),
        request(
            4,
            "tools/call",
            json!({ "name": "read_article", "arguments": { "url": 7 } }),
        ),
    ];

    let responses = run_mcp(&lines, &[]);

    assert_eq!(responses.len(), 5);
    assert_eq!(responses[0]["id"], 1);
    assert_eq!(responses[0]["result"]["serverInfo"]["name"], "lectito-mcp");
    assert_eq!(responses[1]["id"], 2);
    assert_eq!(responses[1]["result"]["tools"].as_array().expect("tools").len(), 2);
    assert_eq!(responses[2]["id"], Value::Null);
    assert_eq!(responses[2]["error"]["code"], -32700);
    assert_eq!(responses[3]["id"], 3);
    assert_eq!(responses[3]["error"]["code"], -32601);
    assert_eq!(responses[4]["id"], 4);
    assert_eq!(responses[4]["error"]["code"], -32602);
}

#[test]
fn read_article_follows_redirects_and_chunks_text() {
    let server = TestServer::new(|path| match path.as_str() {
        "/redirect" => TestResponse::redirect("/article"),
        "/article" => TestResponse::html(article_html()),
        _ => TestResponse::status(404, "text/plain", "not found"),
    });
    let url = server.url("/redirect");

    let responses = run_mcp(
        &[
            tool_call(
                "first",
                "read_article",
                json!({ "url": url, "format": "text", "maxChars": 120 }),
            ),
            tool_call(
                "second",
                "read_article",
                json!({
                    "url": server.url("/article"),
                    "format": "text",
                    "offset": 120,
                    "maxChars": 120
                }),
            ),
        ],
        &[("LECTITO_MCP_ALLOW_PRIVATE_NETWORK", "true")],
    );

    let first = article(&responses[0]);
    assert_eq!(responses[0]["result"]["isError"], false);
    assert!(first["finalUrl"].as_str().expect("final url").ends_with("/article"));
    assert_eq!(first["format"], "text");
    assert_eq!(first["nextOffset"], 120);
    assert_eq!(first["truncated"], true);
    assert!(
        first["content"]
            .as_str()
            .expect("content")
            .contains("fixture paragraph")
    );

    let second = article(&responses[1]);
    assert_eq!(second["offset"], 120);
    assert_eq!(second["nextOffset"], 240);
    assert_ne!(first["content"], second["content"]);
}

#[test]
fn read_article_rejects_private_targets_by_default() {
    let server = TestServer::new(|_| TestResponse::html(article_html()));

    let responses = run_mcp(
        &[tool_call(
            "private",
            "read_article",
            json!({ "url": server.url("/article"), "format": "text" }),
        )],
        &[],
    );

    assert_eq!(responses[0]["result"]["isError"], true);
    assert!(
        responses[0]["result"]["structuredContent"]["error"]
            .as_str()
            .expect("error")
            .contains("private-network targets")
    );
}

#[test]
fn read_article_reports_redirect_limit_content_type_and_size_errors() {
    let server = TestServer::new(|path| match path.as_str() {
        "/r1" => TestResponse::redirect("/r2"),
        "/r2" => TestResponse::redirect("/article"),
        "/plain" => TestResponse::status(200, "text/plain", "plain text"),
        "/large" => TestResponse::html(article_html()),
        "/article" => TestResponse::html(article_html()),
        _ => TestResponse::status(404, "text/plain", "not found"),
    });

    let responses = run_mcp(
        &[
            tool_call("redirect", "read_article", json!({ "url": server.url("/r1") })),
            tool_call("plain", "read_article", json!({ "url": server.url("/plain") })),
            tool_call("large", "read_article", json!({ "url": server.url("/large") })),
        ],
        &[
            ("LECTITO_MCP_ALLOW_PRIVATE_NETWORK", "true"),
            ("LECTITO_MCP_REDIRECT_LIMIT", "1"),
            ("LECTITO_MCP_MAX_FETCH_BYTES", "100"),
        ],
    );

    assert_tool_error_contains(&responses[0], "redirect limit exceeded");
    assert_tool_error_contains(&responses[1], "content type is not supported");
    assert_tool_error_contains(&responses[2], "document is too large");
}

#[test]
fn read_article_reports_timeout() {
    let server = TestServer::new(|path| match path.as_str() {
        "/slow" => TestResponse::html(article_html()).with_delay(Duration::from_millis(1500)),
        _ => TestResponse::status(404, "text/plain", "not found"),
    });

    let responses = run_mcp(
        &[tool_call("slow", "read_article", json!({ "url": server.url("/slow") }))],
        &[
            ("LECTITO_MCP_ALLOW_PRIVATE_NETWORK", "true"),
            ("LECTITO_MCP_REQUEST_TIMEOUT_SECS", "1"),
        ],
    );

    assert_eq!(responses[0]["result"]["isError"], true);
    assert_tool_error_contains(&responses[0], "HTTP request failed");
}
