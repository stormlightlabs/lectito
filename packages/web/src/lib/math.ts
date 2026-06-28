/* eslint-disable unicorn/no-array-for-each  */
/** @module lazy loading of Katex */

import type Katex from "katex";

let katexPromise: Promise<typeof Katex> | undefined;
let stylesheetLoaded = false;

async function loadKatex(): Promise<typeof Katex> {
  katexPromise ??= import("katex").then((mod) => {
    if (!stylesheetLoaded) {
      const link = document.createElement("link");
      link.rel = "stylesheet";
      link.href = new URL("katex/katex.min.css", import.meta.url).href;
      document.head.append(link);
      stylesheetLoaded = true;
    }
    return mod.default;
  });
  return katexPromise;
}

export async function renderMath(container: HTMLElement): Promise<void> {
  const mathSpans = container.querySelectorAll<HTMLElement>("[data-math-style]");
  if (mathSpans.length === 0) return;

  const katex = await loadKatex();

  mathSpans.forEach(function(span) {
    const latex = span.textContent ?? "";
    const displayMode = span.dataset.mathStyle === "display";
    katex.render(latex, span, { displayMode, throwOnError: false });
  });
}
