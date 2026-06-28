use super::*;
use axum::routing::get;
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tower::ServiceExt;

async fn html_server() -> String {
    let app = Router::new().route(
        "/article",
        get(|| async {
            (
                [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                r#"<!doctype html>
                <html>
                  <head><title>Smoke Article</title></head>
                  <body>
                    <main>
                      <h1>Smoke Article</h1>
                      <p>This article has enough real text for the readability
                      smoke test to treat it as article content.</p>
                      <p>Another paragraph keeps the extractor on the article
                      body rather than metadata or page chrome.</p>
                    </main>
                  </body>
                </html>"#,
            )
        }),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://127.0.0.1:{}/article", addr.port())
}

fn test_config() -> Config {
    Config {
        port: 0,
        max_body_bytes: Limit::MaxBodyBytes.into(),
        max_fetch_bytes: Limit::MaxFetchBytes.into(),
        redirect_limit: Limit::Redirect.into(),
        request_timeout_secs: Limit::RequestTimeoutSecs.into(),
        allowed_origins: Vec::new(),
        allow_private_network: true,
        rate_limit: RateLimitConfig::default(),
    }
}

fn json_request(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn body_json(response: Response) -> Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}

#[tokio::test]
async fn healthz_smoke() {
    let response = app(test_config())
        .await
        .oneshot(Request::builder().uri("/healthz").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn transform_returns_json_by_default() {
    let response = app(test_config())
        .await
        .oneshot(json_request(
            "/v1/transform",
            json!({ "html": "<h1>Hello</h1><p>Body</p>" }),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["markdown"], "# Hello\n\nBody");
}

#[tokio::test]
async fn transform_returns_plain_markdown_when_requested() {
    let response = app(test_config())
        .await
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/transform")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::ACCEPT, "text/markdown")
                .body(Body::from(r#"{ "html": "<h1>Hello</h1><p>Body</p>" }"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(body.as_ref(), b"# Hello\n\nBody");
}

#[tokio::test]
async fn extract_smoke() {
    let source = html_server().await;
    let response = app(test_config())
        .await
        .oneshot(json_request(
            "/v1/extract",
            json!({ "url": source, "options": { "charThreshold": 20 }, "diagnostics": true }),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["article"]["title"], "Smoke Article");
    assert!(body["article"]["markdown"].as_str().unwrap().contains("readability"));
    assert!(body["article"]["content"].as_str().unwrap().contains("<"));
    assert!(body["diagnostics"].is_object());
}

#[tokio::test]
async fn evaluate_smoke() {
    let source = html_server().await;
    let response = app(test_config())
        .await
        .oneshot(json_request(
            "/v1/evaluate",
            json!({ "url": source, "options": { "minContentLength": 20, "minScore": 0.0 } }),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["readable"], true);
}

#[tokio::test]
async fn missing_required_field_returns_structured_error() {
    let response = app(test_config())
        .await
        .oneshot(json_request("/v1/transform", json!({})))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(response.headers().get("x-error-code").unwrap(), "invalid_request");
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/json"
    );
    let body = body_json(response).await;
    assert_eq!(body["error"]["code"], "invalid_request");
    assert!(body["error"]["message"].is_string());
}

#[tokio::test]
async fn oversized_body_returns_structured_error() {
    let config = Config {
        max_body_bytes: 16,
        max_fetch_bytes: Limit::MaxFetchBytes.into(),
        redirect_limit: Limit::Redirect.into(),
        request_timeout_secs: Limit::RequestTimeoutSecs.into(),
        allowed_origins: Vec::new(),
        allow_private_network: true,
        rate_limit: RateLimitConfig::default(),
        port: 0,
    };
    let body = json!({ "html": "x".repeat(64) }).to_string();
    let response = app(config)
        .await
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/transform")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::CONTENT_LENGTH, body.len())
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(response.headers().get("x-error-code").unwrap(), "document_too_large");

    let body = body_json(response).await;
    assert_eq!(body["error"]["code"], "document_too_large");
}
