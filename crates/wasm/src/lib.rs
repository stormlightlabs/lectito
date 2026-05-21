//! WASM bindings for Lectito.

use std::sync::Once;

use lectito::{MarkdownOptions, ReadabilityOptions, ReadableOptions};
use serde::Deserialize;
use wasm_bindgen::prelude::*;

static INIT: Once = Once::new();

#[wasm_bindgen(start)]
pub fn start() {
    init();
}

#[wasm_bindgen(js_name = extract)]
pub fn extract_js(html: &str, base_url: Option<String>, options: Option<JsValue>) -> Result<JsValue, JsValue> {
    init();
    let options = readability_options(options)?;
    let article = lectito::extract(html, base_url.as_deref(), &options).map_err(js_error)?;
    serde_wasm_bindgen::to_value(&article).map_err(js_error)
}

#[wasm_bindgen(js_name = extractWithDiagnostics)]
pub fn extract_with_diagnostics_js(
    html: &str, base_url: Option<String>, options: Option<JsValue>,
) -> Result<JsValue, JsValue> {
    init();
    let options = readability_options(options)?;
    let report = lectito::extract_with_diagnostics(html, base_url.as_deref(), &options).map_err(js_error)?;
    serde_wasm_bindgen::to_value(&report).map_err(js_error)
}

#[wasm_bindgen(js_name = isProbablyReadable)]
pub fn is_probably_readable_js(html: &str, options: Option<JsValue>) -> Result<bool, JsValue> {
    init();
    let options = readable_options(options)?;
    lectito::is_probably_readable(html, &options).map_err(js_error)
}

#[wasm_bindgen(js_name = cleanHtml)]
pub fn clean_html_js(html: &str, base_url: Option<String>, options: Option<JsValue>) -> Result<JsValue, JsValue> {
    init();
    let options = clean_html_options(options)?;
    let html = lectito::clean_article_html(html, base_url.as_deref(), &options).map_err(js_error)?;
    serde_wasm_bindgen::to_value(&html).map_err(js_error)
}

#[wasm_bindgen(js_name = htmlToMarkdown)]
pub fn html_to_markdown_js(html: &str) -> String {
    init();
    lectito::html_to_markdown(html)
}

#[wasm_bindgen(js_name = markdownToHtml)]
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

fn clean_html_options(value: Option<JsValue>) -> Result<ReadabilityOptions, JsValue> {
    readability_options(value)
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
    mobile_viewport_width: Option<Option<usize>>,
    classes_to_preserve: Option<Vec<String>>,
    keep_classes: Option<bool>,
    disable_json_ld: Option<bool>,
    link_density_modifier: Option<f32>,
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
