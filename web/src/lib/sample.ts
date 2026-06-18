import type { SampleHtml, SampleUrl } from "./types";

export const sampleUrls: SampleUrl[] = [
  { label: "Wikipedia reference page", url: "https://en.wikipedia.org/wiki/Mozilla" },
  { label: "Factorio table-heavy post", url: "https://factorio.com/blog/post/fff-282" },
  { label: "V8 technical article", url: "https://v8.dev/blog" },
  { label: "Daring Fireball article", url: "https://daringfireball.net/" },
];

// TODO: we should stick to a single sample
export const sampleHtmlFixtures: SampleHtml[] = [{
  label: "Article with chrome",
  html: `<!doctype html>
<html>
  <head>
    <title>Shipping a tiny parser</title>
    <meta property="og:site_name" content="Parser Notes">
    <script>alert("chrome")</script>
  </head>
  <body>
    <nav>Home | Archive | Subscribe</nav>
    <main>
      <article>
        <h1>Shipping a tiny parser</h1>
        <p class="byline">By Ada Parser</p>
        <p>
          This page has <strong>article content</strong>, a
          <a href="/notes">relative link</a>, and enough surrounding chrome to
          make cleanup visible.
        </p>
        <figure>
          <img src="https://placehold.co/960x480/png?text=Parser+diagram" alt="Parser diagram">
          <figcaption>A small diagram preserved through cleanup.</figcaption>
        </figure>
        <pre><code class="language-rust">let markdown = lectito::html_to_markdown(html);</code></pre>
        <table>
          <thead><tr><th>Step</th><th>Output</th></tr></thead>
          <tbody><tr><td>Clean</td><td>HTML</td></tr><tr><td>Convert</td><td>Markdown</td></tr></tbody>
        </table>
      </article>
    </main>
    <aside>Related posts, comments, newsletter signup</aside>
  </body>
</html>`,
}, {
  label: "Code block normalization",
  html: `<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <title>Mintlify code block normalization</title>
</head>
<body>
  <article>
    <h1>Mintlify code block normalization</h1>
    <p>
      This fixture represents documentation pages that wrap highlighted code
      with copy controls, filename chrome, language labels, and line numbers.
      The extractor should keep the article prose and the code sample while
      markdown conversion removes the surrounding interface text.
    </p>
    <div class="code-block not-prose">
      <div class="code-header">
        <span class="filename">client.ts</span>
        <button aria-label="Copy code">Copy</button>
      </div>
      <pre data-language="TypeScript" class="language-ts"><code><span class="line"><span class="line-number">1</span><span class="token keyword">export</span> async function loadArticle(url: string) {</span>
<span class="line"><span class="line-number">2</span>  return client.extract({ url });</span>
<span class="line"><span class="line-number">3</span>}</span></code></pre>
    </div>
    <p>
      The paragraph after the code block gives the readability scorer enough
      natural language context to select the article rather than the navigation
      or code toolbar. It also verifies that prose after a code sample remains
      in the extracted content.
    </p>
  </article>
</body>
</html>`,
}, {
  label: "Inline semantic elements",
  html: `<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <title>Inline semantic elements</title>
  <meta name="description" content="Representative article fixture for inline semantic HTML elements.">
</head>
<body>
  <nav>Site navigation that should not be part of the article.</nav>
  <article>
    <h1>Inline semantic elements</h1>
    <p>
      This article uses inline semantic HTML elements in prose. The reader
      should keep highlighted text, edits, chemical notation, mathematical
      powers, and a small inline vector icon while still extracting a clean
      article body.
    </p>
    <p>
      Search tools can <mark>highlight matching terms</mark>, editors can show
      <del>removed wording</del> next to the revised phrase, and older pages
      sometimes use <s>obsolete labels</s> or <strike>legacy strike text</strike>.
      Chemistry and units need H<sub>2</sub>O and 10<sup>2</sup> m<sup>2</sup>
      to remain distinguishable from normal baseline text.
    </p>
    <p>
      An inline status icon can be represented by SVG
      <svg viewBox="0 0 10 10" role="img" aria-label="complete"><circle cx="5" cy="5" r="4"></circle></svg>
      without losing the graphic element during markdown conversion.
    </p>
    <p>
      The final paragraph gives the extractor more article-like substance. It
      describes why preserving these inline elements matters for downstream
      reading, search, note taking, and archival output.
    </p>
  </article>
</body>
</html>`,
}];

export const sampleHtml = sampleHtmlFixtures[0]?.html ?? "";
