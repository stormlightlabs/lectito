// TODO: replace '/images/parser.png' with placehold.co image
export const sampleHtml = `<!doctype html>
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
          <img src="/images/parser.png" alt="Parser diagram">
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
</html>`;
