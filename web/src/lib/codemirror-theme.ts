import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import type { Extension } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { tags as t } from "@lezer/highlight";

type Base16 = {
  /** base00 is the default editor background. */
  base00: string;
  /** base01 is for gutters/status surfaces; base04 is status text. */
  base01: string;
  /** base02 is the Base16 selection color. */
  base02: string;
  /**
   * base03 covers comments, invisibles, and line highlighting.
   *
   * Use it at low opacity so the active line does not compete with
   * the selection color.
   */
  base03: string;
  base04: string;
  /** base05 is the default text, caret, delimiter, and operator color. */
  base05: string;
  base06: string;
  base07: string;
  /** base08: variables, XML/HTML tags, markup link text, lists, and deletions. */
  base08: string;
  /** base09: integers, booleans, constants, XML/HTML attributes, and link URLs. */
  base09: string;
  /** base0A: classes, markup bold, and search-highlight-like emphasis. */
  base0A: string;
  /** base0B: strings, inherited classes, markup code, and insertions. */
  base0B: string;
  /** base0C: support names, regular expressions, escape characters, and quotes. */
  base0C: string;
  /** base0D: functions, methods, IDs, and headings. */
  base0D: string;
  /** base0E: keywords, storage, selectors, changed diffs, and italic markup. */
  base0E: string;
  /** base0F: deprecated syntax and embedded-language delimiters. */
  base0F: string;
};

/**
 * @system: "base16"
 * @name: "Da One Paper"
 * @author: "NNB (https://github.com/NNBnh)"
 * @variant: "light"
 */
const daOnePaper: Base16 = {
  base00: "#faf0dc",
  base01: "#c8c8c8",
  base02: "#888888",
  base03: "#585858",
  base04: "#282828",
  base05: "#181818",
  base06: "#000000",
  base07: "#000000",
  base08: "#de5d6e",
  base09: "#ff9470",
  base0A: "#b3684f",
  base0B: "#76a85d",
  base0C: "#64b5a7",
  base0D: "#5890f8",
  base0E: "#c173d1",
  base0F: "#b3684f",
} as const;

const theme = EditorView.theme({
  "&": { backgroundColor: daOnePaper.base00, color: daOnePaper.base05 },
  "&.cm-focused": { outline: "none" },
  ".cm-content": { caretColor: daOnePaper.base05 },
  ".cm-cursor, .cm-dropCursor": { borderLeftColor: daOnePaper.base05 },
  ".cm-selectionBackground, &.cm-focused .cm-selectionBackground, ::selection": { backgroundColor: daOnePaper.base02 },
  ".cm-activeLine": { backgroundColor: `${daOnePaper.base03}20` },
  ".cm-gutters": { backgroundColor: daOnePaper.base01, borderRightColor: daOnePaper.base02, color: daOnePaper.base04 },
  ".cm-activeLineGutter": { backgroundColor: daOnePaper.base02, color: daOnePaper.base07 },
  ".cm-lineNumbers": { backgroundColor: "var(--paper-muted)", color: daOnePaper.base07 },
  ".cm-gutterElement": { backgroundColor: "var(--paper-muted)", color: daOnePaper.base07 },
  ".cm-foldPlaceholder": {
    backgroundColor: daOnePaper.base01,
    borderColor: daOnePaper.base02,
    color: daOnePaper.base04,
  },
  ".cm-panels, .cm-search": { backgroundColor: daOnePaper.base01, color: daOnePaper.base05 },
  ".cm-tooltip": { backgroundColor: daOnePaper.base00, borderColor: daOnePaper.base02, color: daOnePaper.base05 },
  ".cm-tooltip-autocomplete > ul > li[aria-selected]": { backgroundColor: daOnePaper.base02, color: daOnePaper.base07 },
  ".cm-matchingBracket, .cm-nonmatchingBracket": {
    backgroundColor: `${daOnePaper.base0A}30`,
    color: daOnePaper.base07,
  },
  ".cm-diagnostic-error": { borderLeftColor: daOnePaper.base08 },
  ".cm-diagnostic-warning": { borderLeftColor: daOnePaper.base09 },
}, { dark: false });

const highlightStyle = HighlightStyle.define([
  { tag: [t.comment, t.lineComment, t.blockComment], color: daOnePaper.base03, fontStyle: "italic" },
  { tag: [t.variableName, t.self, t.tagName, t.deleted, t.list], color: daOnePaper.base08 },
  { tag: [t.number, t.integer, t.bool, t.null, t.atom, t.attributeName, t.url], color: daOnePaper.base09 },
  { tag: [t.className, t.definition(t.typeName), t.heading, t.strong], color: daOnePaper.base0A },
  { tag: t.strong, fontWeight: "600" },
  { tag: [t.string, t.special(t.string), t.inserted, t.monospace], color: daOnePaper.base0B },
  { tag: [t.regexp, t.escape, t.character, t.special(t.variableName), t.quote], color: daOnePaper.base0C },
  {
    tag: [t.function(t.variableName), t.function(t.propertyName), t.definition(t.propertyName), t.labelName],
    color: daOnePaper.base0D,
  },
  {
    tag: [t.keyword, t.operatorKeyword, t.modifier, t.controlKeyword, t.processingInstruction, t.changed, t.emphasis],
    color: daOnePaper.base0E,
  },
  { tag: t.emphasis, fontStyle: "italic" },
  { tag: [t.invalid, t.meta], color: daOnePaper.base0F },
  { tag: [t.operator, t.punctuation, t.bracket, t.separator, t.derefOperator], color: daOnePaper.base05 },
]);

export const daOnePaperLightCodeMirrorTheme: Extension = [theme, syntaxHighlighting(highlightStyle)];
