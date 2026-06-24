use kuchiki::NodeRef;
use kuchiki::traits::TendrilSink;

use super::RenderContext;
use crate::{dom, patterns};

pub(crate) fn normalize_code_markup(root: &NodeRef) {
    remove_code_chrome(root);
    normalize_code_tables(root);
    normalize_standalone_code_containers(root);
    normalize_pre_blocks(root);
}

pub(super) fn render_code_block(node: &NodeRef, _ctx: RenderContext) -> String {
    CodeBlock::from_node(node).fenced_code_block()
}

pub(super) fn render_code_table(node: &NodeRef, _ctx: RenderContext) -> Option<String> {
    Some(CodeBlock::from_table(node)?.fenced_code_block())
}

pub(super) fn render_code_container(node: &NodeRef, ctx: RenderContext) -> Option<String> {
    if !is_standalone_code_container(node) {
        return None;
    }

    if let Some(pre) = dom::select_nodes(node, "pre").into_iter().next() {
        return Some(render_code_block(&pre, ctx));
    }

    let language = get_lang_id(node);
    let code = code_text(node);
    if code.trim().is_empty() {
        return None;
    }

    Some(CodeBlock::new(language, code).fenced_code_block())
}

fn code_text(node: &NodeRef) -> String {
    let line_nodes = code_line_nodes(node);
    if !line_nodes.is_empty() {
        return line_nodes
            .iter()
            .map(|line| code_text_fragment(line).trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n");
    }
    code_text_fragment(node)
}

fn code_line_nodes(root: &NodeRef) -> Vec<NodeRef> {
    dom::select_nodes(root, "*")
        .into_iter()
        .filter(is_code_line_node)
        .filter(|node| !has_code_line_ancestor(node, root))
        .collect()
}

fn has_code_line_ancestor(node: &NodeRef, root: &NodeRef) -> bool {
    let root_id = dom::node_id(root);
    let mut parent = node.parent();
    while let Some(ancestor) = parent {
        if dom::node_id(&ancestor) == root_id {
            return false;
        }
        if is_code_line_node(&ancestor) {
            return true;
        }
        parent = ancestor.parent();
    }
    false
}

fn code_text_fragment(node: &NodeRef) -> String {
    if let Some(text) = node.as_text() {
        return text.borrow().to_string();
    }

    if is_highlighter_chrome(node) || is_line_number_node(node) {
        return String::new();
    }

    if dom::node_name(node) == "br" {
        return "\n".to_string();
    }

    let mut output = String::new();
    for child in node.children() {
        output.push_str(&code_text_fragment(&child));
    }
    output
}

fn get_lang_id(node: &NodeRef) -> Option<String> {
    for attr in ["data-lang", "data-language", "language", "lang"] {
        if let Some(language) = dom::attr(node, attr).and_then(|value| clean_lang_id(&value)) {
            return Some(language);
        }
    }

    let class = dom::attr(node, "class")?;
    language_from_class(&class)
}

fn ancestor_lang_id(node: &NodeRef) -> Option<String> {
    node.ancestors()
        .skip(1)
        .take(4)
        .find_map(|ancestor| get_lang_id(&ancestor))
}

fn language_from_class(class: &str) -> Option<String> {
    class
        .split_whitespace()
        .find_map(clean_language_class)
        .or_else(|| class.split_once("brush:").and_then(|(_, value)| clean_lang_id(value)))
}

fn clean_language_class(class: &str) -> Option<String> {
    let value = class
        .strip_prefix("language-")
        .or_else(|| class.strip_prefix("lang-"))
        .or_else(|| class.strip_prefix("highlight-source-"))
        .or_else(|| class.strip_prefix("brush:"))
        .or_else(|| class.strip_prefix("syntax--"))?;
    clean_lang_id(value)
}

fn clean_lang_id(value: &str) -> Option<String> {
    let value = value
        .trim()
        .trim_matches(|c: char| c == ';' || c == ':' || c == ',' || c == '"' || c == '\'')
        .to_ascii_lowercase();
    let value = value
        .split_whitespace()
        .find(|part| !matches!(*part, "notranslate" | "highlight" | "syntax"))
        .unwrap_or_default()
        .trim_matches(|c: char| c == ';' || c == ':' || c == ',' || c == '"' || c == '\'');
    if value.is_empty() || matches!(value, "none" | "text" | "plain" | "plaintext") {
        None
    } else {
        Some(value.to_string())
    }
}

fn is_standalone_code_container(node: &NodeRef) -> bool {
    let class = dom::attr(node, "class").unwrap_or_default().to_ascii_lowercase();
    has_class_token(node, "codemirror-code")
        || has_class_token(node, "cm-content")
        || class.split_whitespace().any(|part| part == "sourcecode")
        || class.contains("react-syntax-highlighter")
        || class.contains("highlight")
        || class.contains("codeblock")
        || class.contains("code-block")
        || class.contains("codehilite")
}

fn is_code_line_node(node: &NodeRef) -> bool {
    if is_line_number_node(node) || is_highlighter_chrome(node) {
        return false;
    }
    let class = dom::attr(node, "class").unwrap_or_default();
    has_class_token_value(&class, "cm-line")
        || has_class_token_value(&class, "token-line")
        || has_class_token_value(&class, "line")
        || has_class_token_value(&class, "cl")
        || has_class_token_value(&class, "code-line")
        || dom::attr(node, "data-line").is_some()
}

fn is_line_number_node(node: &NodeRef) -> bool {
    let class = dom::attr(node, "class").unwrap_or_default();
    [
        "lineno",
        "linenos",
        "line-number",
        "line-numbers-rows",
        "line-numbers",
        "gutter",
        "rouge-gutter",
        "code-gutter",
        "highlight-line",
        "ln",
    ]
    .iter()
    .any(|token| has_class_token_value(&class, token))
}

pub(super) fn is_highlighter_chrome(node: &NodeRef) -> bool {
    let tag = dom::node_name(node);
    let class = dom::attr(node, "class").unwrap_or_default().to_ascii_lowercase();
    let id = dom::attr(node, "id").unwrap_or_default().to_ascii_lowercase();
    let role = dom::attr(node, "role").unwrap_or_default().to_ascii_lowercase();
    let chrome = [
        "toolbar",
        "copy-button",
        "clipboard",
        "filename",
        "language-name",
        "code-header",
        "code-title",
        "codemirror-gutters",
        "cm-gutters",
        "gutter-wrapper",
    ];
    if class.contains("code-toolbar") && dom::select_nodes(node, "pre").is_empty() {
        return true;
    }
    if chrome.iter().any(|needle| {
        if *needle == "toolbar" {
            has_class_token_value(&class, needle) || id == "toolbar"
        } else {
            class.contains(needle) || id.contains(needle)
        }
    }) {
        return true;
    }
    if role == "toolbar" {
        return true;
    }
    tag == "button" && is_copy_label(node)
}

fn is_copy_label(node: &NodeRef) -> bool {
    let label = dom::attr(node, "aria-label")
        .or_else(|| dom::attr(node, "title"))
        .unwrap_or_else(|| node.text_contents());
    matches!(
        patterns::normalize_spaces(&label).to_ascii_lowercase().as_str(),
        "copy" | "copy code" | "copied" | "copied!"
    )
}

fn has_class_token(node: &NodeRef, token: &str) -> bool {
    dom::attr(node, "class").is_some_and(|class| has_class_token_value(&class, token))
}

fn has_class_token_value(class: &str, token: &str) -> bool {
    class.split_whitespace().any(|part| part.eq_ignore_ascii_case(token))
}

fn remove_code_chrome(root: &NodeRef) {
    for node in dom::select_nodes(root, "*") {
        if is_highlighter_chrome(&node) || is_line_number_node(&node) {
            node.detach();
        }
    }
}

fn normalize_code_tables(root: &NodeRef) {
    for table in dom::select_nodes(root, "table") {
        let Some(code) = CodeBlock::from_table(&table) else {
            continue;
        };
        replace_with_pre(&table, code.language.as_deref(), &code.text);
    }
}

fn normalize_standalone_code_containers(root: &NodeRef) {
    for node in dom::select_nodes(root, "div") {
        if dom::select_nodes(&node, "pre").is_empty() && is_standalone_code_container(&node) {
            let code = CodeBlock { language: get_lang_id(&node), text: code_text(&node) };
            if !code.text.trim().is_empty() {
                replace_with_pre(&node, code.language.as_deref(), &code.text);
            }
        }
    }
}

fn normalize_pre_blocks(root: &NodeRef) {
    for pre in dom::select_nodes(root, "pre") {
        let code_node = dom::select_nodes(&pre, "code").into_iter().next();
        let language = get_lang_id(&pre)
            .or_else(|| code_node.as_ref().and_then(get_lang_id))
            .or_else(|| ancestor_lang_id(&pre));
        let text = code_text(code_node.as_ref().unwrap_or(&pre));
        replace_with_pre(&pre, language.as_deref(), text.trim_matches('\n'));
    }
}

struct CodeBlock {
    language: Option<String>,
    text: String,
}

impl CodeBlock {
    fn new(language: Option<String>, text: String) -> Self {
        Self { language, text }
    }

    fn from_node(node: &NodeRef) -> Self {
        let code_node = dom::select_nodes(node, "code").into_iter().next();
        Self::new(
            get_lang_id(node)
                .or_else(|| code_node.as_ref().and_then(get_lang_id))
                .or_else(|| ancestor_lang_id(node)),
            code_text(code_node.as_ref().unwrap_or(node)),
        )
    }

    fn from_table(node: &NodeRef) -> Option<CodeBlock> {
        if !(has_class_token(node, "highlighttable")
            || has_class_token(node, "rouge-table")
            || has_class_token(node, "highlight")
            || !dom::select_nodes(node, "td.linenos, td.rouge-gutter, td.gutter, td.code, td.rouge-code").is_empty())
        {
            return None;
        }

        let code_cell = dom::select_nodes(node, "td.code, td.rouge-code")
            .into_iter()
            .next()
            .or_else(|| {
                dom::select_nodes(node, "td")
                    .into_iter()
                    .filter(|cell| !is_line_number_node(cell))
                    .max_by_key(|cell| code_text(cell).trim().len())
            })?;
        let text = code_text(&code_cell);
        (!text.trim().is_empty())
            .then(|| CodeBlock { language: get_lang_id(node).or_else(|| get_lang_id(&code_cell)), text })
    }

    fn code(&self) -> String {
        self.text.trim_matches('\n').to_string()
    }

    fn language(&self) -> &str {
        self.language.as_deref().unwrap_or_default()
    }

    fn fenced_code_block(&self) -> String {
        let longest_run = self
            .code()
            .lines()
            .flat_map(|line| line.split(|c| c != '`').map(str::len))
            .max()
            .unwrap_or(0);

        format!(
            r#"{fence}{lang}
{code}
{fence}"#,
            lang = self.language(),
            fence = "`".repeat(longest_run.max(3)),
            code = self.code()
        )
    }
}

fn replace_with_pre(node: &NodeRef, language: Option<&str>, text: &str) {
    let fragment = kuchiki::parse_html().one("<html><body><pre><code></code></pre></body></html>");
    let Some(pre) = dom::select_nodes(&fragment, "pre").into_iter().next() else {
        return;
    };
    let Some(code) = dom::select_nodes(&pre, "code").into_iter().next() else {
        return;
    };
    if let Some(language) = language {
        dom::set_attr(&pre, "data-language", language);
        dom::set_attr(&code, "data-language", language);
    }
    code.append(NodeRef::new_text(text.to_string()));
    node.insert_before(pre);
    node.detach();
}

#[cfg(test)]
mod tests {
    use crate::markdown::html_to_markdown;

    #[test]
    fn preserves_code_fence_languages() {
        let markdown = html_to_markdown(
            r#"<pre data-lang="Rust"><code class="language-rust"><span class="token keyword">fn</span> main() {}</code></pre>"#,
        );

        assert!(markdown.contains("```rust\nfn main() {}\n```"), "{markdown}");
    }

    #[test]
    fn strips_code_block_chrome_and_line_numbers() {
        let markdown = html_to_markdown(
            r#"<div class="code-toolbar"><pre class="language-js"><code><span class="line"><span class="line-number">1</span><span class="token keyword">const</span> value = 1;</span><span class="line"><span class="line-number">2</span>console.log(value);</span></code></pre><div class="toolbar"><button>Copy code</button></div></div>"#,
        );

        assert!(
            markdown.contains("```js\nconst value = 1;\nconsole.log(value);\n```"),
            "{markdown}"
        );
        assert!(!markdown.contains("Copy"), "{markdown}");
        assert!(!markdown.contains("\n1const"), "{markdown}");
    }

    #[test]
    fn normalizes_pygments_rouge_code_tables() {
        let markdown = html_to_markdown(
            r#"<table class="highlighttable language-python"><tr><td class="linenos"><div>1<br>2</div></td><td class="code"><pre><span></span>def hello():
    return "world"</pre></td></tr></table>"#,
        );

        assert!(
            markdown.contains("```python\ndef hello():\n    return \"world\"\n```"),
            "{markdown}"
        );
        assert!(!markdown.contains("| --- |"), "{markdown}");
        assert!(!markdown.contains("1\n2"), "{markdown}");
    }

    #[test]
    fn normalizes_codemirror_line_containers() {
        let markdown = html_to_markdown(
            r#"<div class="cm-content language-ts"><div class="cm-line"><span class="cm-keyword">type</span> User = { id: string };</div><div class="cm-line">export default User;</div></div>"#,
        );

        assert!(
            markdown.contains("```ts\ntype User = { id: string };\nexport default User;\n```"),
            "{markdown}"
        );
    }

    #[test]
    fn inherits_language_from_code_block_wrapper() {
        let markdown = html_to_markdown(
            r#"<div language="jsx" numberoflines="4"><div data-component-part="code-block-root"><div><pre><code>export const Button = () =&gt; (
  &lt;button&gt;Save&lt;/button&gt;
);</code></pre></div></div></div>"#,
        );

        assert!(
            markdown.contains("```jsx\nexport const Button = () => (\n  <button>Save</button>\n);\n```"),
            "{markdown}"
        );
    }

    #[test]
    fn normalizes_mdn_language_classes_and_labels() {
        let markdown = html_to_markdown(
            r#"<div class="code-example"><div class="example-header"><span class="language-name">JavaScript</span></div><pre class="brush: js notranslate"><code>const proxy = new Proxy(target, handler);</code></pre></div>"#,
        );

        assert!(
            markdown.contains("```js\nconst proxy = new Proxy(target, handler);\n```"),
            "{markdown}"
        );
        assert!(!markdown.contains("JavaScript"), "{markdown}");
        assert!(!markdown.contains("notranslate"), "{markdown}");
    }
}
