//! WASM bindings for Lectito.

use std::sync::Once;

use lectito::{MarkdownOptions, MediaRetention, ReadabilityOptions, ReadableOptions};
use serde::Deserialize;
use wasm_bindgen::prelude::*;

static INIT: Once = Once::new();

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &'static str = include_str!("types.d.ts");

#[wasm_bindgen(start)]
pub fn start() {
    init();
}

#[wasm_bindgen(js_name = extract, skip_typescript)]
pub fn extract_js(html: &str, base_url: Option<String>, options: Option<JsValue>) -> Result<JsValue, JsValue> {
    init();
    let options = readability_options(options)?;
    let article = lectito::extract(html, base_url.as_deref(), &options).map_err(js_error)?;
    serde_wasm_bindgen::to_value(&article).map_err(js_error)
}

#[wasm_bindgen(js_name = extractWithDiagnostics, skip_typescript)]
pub fn extract_with_diagnostics_js(
    html: &str, base_url: Option<String>, options: Option<JsValue>,
) -> Result<JsValue, JsValue> {
    init();
    let options = readability_options(options)?;
    let report = lectito::extract_with_diagnostics(html, base_url.as_deref(), &options).map_err(js_error)?;
    serde_wasm_bindgen::to_value(&report).map_err(js_error)
}

#[wasm_bindgen(js_name = isProbablyReadable, skip_typescript)]
pub fn is_probably_readable_js(html: &str, options: Option<JsValue>) -> Result<bool, JsValue> {
    init();
    let options = readable_options(options)?;
    lectito::is_probably_readable(html, &options).map_err(js_error)
}

#[wasm_bindgen(js_name = cleanHtml, skip_typescript)]
pub fn clean_html_js(html: &str, base_url: Option<String>, options: Option<JsValue>) -> Result<JsValue, JsValue> {
    init();
    let options = readability_options(options)?;
    let html = lectito::clean_article_html(html, base_url.as_deref(), &options).map_err(js_error)?;
    serde_wasm_bindgen::to_value(&html).map_err(js_error)
}

#[wasm_bindgen(js_name = htmlToMarkdown, skip_typescript)]
pub fn html_to_markdown_js(html: &str) -> String {
    init();
    lectito::html_to_markdown(html)
}

#[wasm_bindgen(js_name = markdownToHtml, skip_typescript)]
pub fn markdown_to_html_js(markdown: &str, options: Option<JsValue>) -> Result<String, JsValue> {
    init();
    let options = markdown_options(options)?;
    Ok(lectito::markdown_to_html(markdown, &options))
}

fn init() {
    INIT.call_once(console_error_panic_hook::set_once);
}

fn readability_options(value: Option<JsValue>) -> Result<ReadabilityOptions, JsValue> {
    let Some(value) = present_value(value) else {
        return Ok(ReadabilityOptions::default());
    };
    let options: ReadabilityOptionsDto = serde_wasm_bindgen::from_value(value).map_err(js_error)?;
    Ok(options.into_options())
}

fn readable_options(value: Option<JsValue>) -> Result<ReadableOptions, JsValue> {
    let Some(value) = present_value(value) else {
        return Ok(ReadableOptions::default());
    };
    let options: ReadableOptionsDto = serde_wasm_bindgen::from_value(value).map_err(js_error)?;
    Ok(options.into_options())
}

fn markdown_options(value: Option<JsValue>) -> Result<MarkdownOptions, JsValue> {
    let Some(value) = present_value(value) else {
        return Ok(MarkdownOptions::default());
    };
    let options: MarkdownOptionsDto = serde_wasm_bindgen::from_value(value).map_err(js_error)?;
    Ok(options.into_options())
}

fn present_value(value: Option<JsValue>) -> Option<JsValue> {
    value.filter(|value| !value.is_null() && !value.is_undefined())
}

fn js_error(error: impl ToString) -> JsValue {
    js_sys::Error::new(&error.to_string()).into()
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct ReadabilityOptionsDto {
    max_elems_to_parse: Option<Option<usize>>,
    nb_top_candidates: Option<usize>,
    char_threshold: Option<usize>,
    content_selector: Option<Option<String>>,
    site_profiles: Option<Vec<String>>,
    mobile_viewport_width: Option<Option<usize>>,
    classes_to_preserve: Option<Vec<String>>,
    keep_classes: Option<bool>,
    disable_json_ld: Option<bool>,
    link_density_modifier: Option<f32>,
    media_retention: Option<MediaRetention>,
}

impl ReadabilityOptionsDto {
    fn into_options(self) -> ReadabilityOptions {
        let mut options = ReadabilityOptions::default();
        if let Some(value) = self.max_elems_to_parse {
            options.max_elems_to_parse = value;
        }
        if let Some(value) = self.nb_top_candidates {
            options.nb_top_candidates = value;
        }
        if let Some(value) = self.char_threshold {
            options.char_threshold = value;
        }
        if let Some(value) = self.content_selector {
            options.content_selector = value;
        }
        if let Some(value) = self.site_profiles {
            options.site_profiles = value;
        }
        if let Some(value) = self.mobile_viewport_width {
            options.mobile_viewport_width = value;
        }
        if let Some(value) = self.classes_to_preserve {
            options.classes_to_preserve = value;
        }
        if let Some(value) = self.keep_classes {
            options.keep_classes = value;
        }
        if let Some(value) = self.disable_json_ld {
            options.disable_json_ld = value;
        }
        if let Some(value) = self.link_density_modifier {
            options.link_density_modifier = value;
        }
        if let Some(value) = self.media_retention {
            options.media_retention = value;
        }
        options
    }
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct ReadableOptionsDto {
    min_content_length: Option<usize>,
    min_score: Option<f32>,
}

impl ReadableOptionsDto {
    fn into_options(self) -> ReadableOptions {
        let mut options = ReadableOptions::default();
        if let Some(value) = self.min_content_length {
            options.min_content_length = value;
        }
        if let Some(value) = self.min_score {
            options.min_score = value;
        }
        options
    }
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct MarkdownOptionsDto {
    gfm: Option<bool>,
    footnotes: Option<bool>,
    math: Option<bool>,
    allow_raw_html: Option<bool>,
}

impl MarkdownOptionsDto {
    fn into_options(self) -> MarkdownOptions {
        let mut options = MarkdownOptions::default();
        if let Some(value) = self.gfm {
            options.gfm = value;
        }
        if let Some(value) = self.footnotes {
            options.footnotes = value;
        }
        if let Some(value) = self.math {
            options.math = value;
        }
        if let Some(value) = self.allow_raw_html {
            options.allow_raw_html = value;
        }
        options
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use js_sys::{Object, Reflect};
    use wasm_bindgen::JsValue;
    use wasm_bindgen_test::*;

    use super::*;

    const HTML: &str = r#"
        <html>
            <head>
                <title>WASM Story</title>
                <meta name="author" content="Ada Example">
            </head>
            <body>
                <article>
                    <h1>WASM Story</h1>
                    <p>This article has enough text, punctuation, and concrete detail to pass extraction in a WebAssembly runtime.</p>
                    <p>The second paragraph checks that Markdown, plain text, and metadata cross the JavaScript boundary.</p>
                    <figure><img src="/image.jpg" alt="Demo"></figure>
                </article>
            </body>
        </html>
    "#;

    #[wasm_bindgen_test]
    fn extract_returns_article_shape() {
        let article = extract_js(HTML, Some("https://example.com/post".to_string()), None).expect("extract succeeds");

        assert_eq!(prop_string(&article, "title").as_deref(), Some("WASM Story"));
        assert_eq!(prop_string(&article, "byline").as_deref(), Some("Ada Example"));
        assert!(
            prop_string(&article, "text_content")
                .expect("text content")
                .contains("second paragraph checks")
        );
        assert!(
            prop_string(&article, "markdown")
                .expect("markdown")
                .contains("second paragraph checks")
        );
        assert_eq!(prop_string(&article, "domain").as_deref(), Some("example.com"));
    }

    #[wasm_bindgen_test]
    fn extract_with_diagnostics_returns_report() {
        let options = object(&[
            ("charThreshold", JsValue::from_f64(0.0)),
            ("contentSelector", JsValue::from_str("article")),
            ("keepClasses", JsValue::TRUE),
        ]);

        let report = extract_with_diagnostics_js(HTML, Some("https://example.com/post".to_string()), Some(options))
            .expect("diagnostic extract succeeds");

        let article = Reflect::get(&report, &JsValue::from_str("article")).expect("article property");
        assert_eq!(prop_string(&article, "title").as_deref(), Some("WASM Story"));

        let diagnostics = Reflect::get(&report, &JsValue::from_str("diagnostics")).expect("diagnostics property");
        assert!(!diagnostics.is_null() && !diagnostics.is_undefined());
    }

    #[wasm_bindgen_test]
    fn clean_html_honors_media_retention_option() {
        let none = object(&[
            ("charThreshold", JsValue::from_f64(0.0)),
            ("contentSelector", JsValue::from_str("article")),
            ("mediaRetention", JsValue::from_str("none")),
        ]);
        let all = object(&[
            ("charThreshold", JsValue::from_f64(0.0)),
            ("contentSelector", JsValue::from_str("article")),
            ("mediaRetention", JsValue::from_str("all")),
        ]);

        let without_media = clean_html_js(HTML, Some("https://example.com/post".to_string()), Some(none))
            .expect("clean succeeds")
            .as_string()
            .expect("clean html string");
        let with_media = clean_html_js(HTML, Some("https://example.com/post".to_string()), Some(all))
            .expect("clean succeeds")
            .as_string()
            .expect("clean html string");

        assert!(!without_media.contains("<img"));
        assert!(with_media.contains("https://example.com/image.jpg"));
    }

    #[wasm_bindgen_test]
    fn readable_and_markdown_helpers_work() {
        let readable_options = object(&[
            ("minContentLength", JsValue::from_f64(40.0)),
            ("minScore", JsValue::from_f64(1.0)),
        ]);

        assert!(is_probably_readable_js(HTML, Some(readable_options)).expect("readability check"));

        let markdown = html_to_markdown_js(r#"<h1>Hello</h1><p>A <strong>bold</strong> move.</p>"#);
        assert!(markdown.contains("# Hello"));
        assert!(markdown.contains("**bold**"));

        let html = markdown_to_html_js("A | B\n-- | --\n1 | 2", None).expect("markdown renders");
        assert!(html.contains("<table>"));
    }

    #[wasm_bindgen_test]
    fn invalid_options_throw_js_errors() {
        let options = object(&[("mediaRetention", JsValue::from_str("everything"))]);
        let error = extract_js(HTML, None, Some(options)).expect_err("invalid option should fail");
        assert!(
            error_message(&error)
                .expect("error message")
                .contains("unknown variant")
        );

        let error = extract_js(HTML, Some("not a url".to_string()), None).expect_err("invalid base url should fail");
        assert!(
            error_message(&error)
                .expect("error message")
                .contains("invalid base URL")
        );
    }

    fn object(entries: &[(&str, JsValue)]) -> JsValue {
        let object = Object::new();
        for (key, value) in entries {
            Reflect::set(&object, &JsValue::from_str(key), value).expect("set property");
        }
        object.into()
    }

    fn prop_string(value: &JsValue, property: &str) -> Option<String> {
        Reflect::get(value, &JsValue::from_str(property)).ok()?.as_string()
    }

    fn error_message(value: &JsValue) -> Option<String> {
        Reflect::get(value, &JsValue::from_str("message"))
            .ok()
            .and_then(|message| message.as_string())
    }
}
