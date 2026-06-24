# WASM To-Do

## Browser Safety

- Keep `sanitize_html` out of core unless the project adopts a real sanitizer
  policy.
- Browser integrations should use DOMPurify or similar before rendering
  arbitrary HTML.

## Release Readiness

- Add WASM smoke tests for extraction, HTML-to-Markdown, and Markdown-to-HTML.
- Measure release package size after adding real exports.
- Add package metadata and a copied license file to the generated WASM package
  before treating it as publishable.
