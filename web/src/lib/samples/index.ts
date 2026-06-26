// TODO: add 2–3 curated real-page captures (e.g. an RFC, an MDN doc,
// paulgraham.com/makersschedule) alongside these synthetic fixtures.
//
// Each should be created as a proper fixture under
// crates/fixtures/samples/test-pages/ with verified expected.html and
// expected-metadata.json, then copied here and added to gallerySamples.

import codeblocksCodemirror from "./codeblocks-codemirror.html?raw";
import codeblocksMdn from "./codeblocks-mdn.html?raw";
import codeblocksMintlifyTabs from "./codeblocks-mintlify-tabs.html?raw";
import codeblocksMintlify from "./codeblocks-mintlify.html?raw";
import elementsInlineSemantic from "./elements-inline-semantic.html?raw";
import footnotesGoogleDocs from "./footnotes-google-docs.html?raw";
import footnotesNumericAnchor from "./footnotes-numeric-anchor.html?raw";
import imagesSrcset from "./images-srcset.html?raw";
import mathKatex from "./math-katex.html?raw";
import mathMathjaxSvg from "./math-mathjax-svg.html?raw";
import mediaEmbeddedVideos from "./media-embedded-videos.html?raw";
import tablesComplex from "./tables-complex.html?raw";
import tablesData from "./tables-data.html?raw";

export type SampleCategory = "Math" | "Code" | "Footnotes" | "Tables" | "Media" | "Elements";

export type GallerySample = { id: string; label: string; description: string; category: SampleCategory; html: string };

export const sampleCategories: SampleCategory[] = ["Math", "Code", "Footnotes", "Tables", "Media", "Elements"];

export const gallerySamples: GallerySample[] = [{
  id: "katex",
  label: "KaTeX math",
  description: "Inline and block TeX annotations in KaTeX output.",
  category: "Math",
  html: mathKatex,
}, {
  id: "mathjax-svg",
  label: "MathJax SVG",
  description: "MathJax SVG output with assistive MathML.",
  category: "Math",
  html: mathMathjaxSvg,
}, {
  id: "codeblocks-mintlify",
  label: "Mintlify code blocks",
  description: "Copy chrome, filename, line numbers around code.",
  category: "Code",
  html: codeblocksMintlify,
}, {
  id: "codeblocks-mintlify-tabs",
  label: "Mintlify tabbed code",
  description: "Tabbed language switcher wrapping code samples.",
  category: "Code",
  html: codeblocksMintlifyTabs,
}, {
  id: "codeblocks-mdn",
  label: "MDN code blocks",
  description: "MDN-style syntax highlighting with language labels.",
  category: "Code",
  html: codeblocksMdn,
}, {
  id: "codeblocks-codemirror",
  label: "CodeMirror code",
  description: "Editor-shaped code with gutter line containers.",
  category: "Code",
  html: codeblocksCodemirror,
}, {
  id: "footnotes-google-docs",
  label: "Google Docs footnotes",
  description: "StackPrinter/Google Docs ftnt footnote anchors.",
  category: "Footnotes",
  html: footnotesGoogleDocs,
}, {
  id: "footnotes-numeric-anchor",
  label: "Numeric anchor footnotes",
  description: "Stack Exchange numeric anchor footnote IDs.",
  category: "Footnotes",
  html: footnotesNumericAnchor,
}, {
  id: "tables-complex",
  label: "Complex tables",
  description: "RFC registry tables with spanning header cells.",
  category: "Tables",
  html: tablesComplex,
}, {
  id: "tables-data",
  label: "Data tables",
  description: "Wikipedia-style data table with MathML fallback.",
  category: "Tables",
  html: tablesData,
}, {
  id: "images-srcset",
  label: "Responsive srcset images",
  description: "Multiple width candidates with a sizes attribute.",
  category: "Media",
  html: imagesSrcset,
}, {
  id: "media-embedded-videos",
  label: "Embedded videos",
  description: "MDN-style embedded video iframes in figures.",
  category: "Media",
  html: mediaEmbeddedVideos,
}, {
  id: "elements-inline-semantic",
  label: "Inline semantic elements",
  description: "mark, del, sub, sup, SVG, and inline media in prose.",
  category: "Elements",
  html: elementsInlineSemantic,
}];

export function findSample(id: string | undefined): GallerySample | undefined {
  if (id) {
    return gallerySamples.find((sample) => sample.id === id);
  } else {
    return;
  }
}
