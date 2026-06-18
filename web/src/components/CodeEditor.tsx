import { catppuccinLatte } from "@catppuccin/codemirror";
import { indentWithTab } from "@codemirror/commands";
import { html } from "@codemirror/lang-html";
import { markdown } from "@codemirror/lang-markdown";
import { Compartment, EditorState } from "@codemirror/state";
import { EditorView, keymap, lineNumbers } from "@codemirror/view";
import { createEffect, createSignal, onCleanup } from "solid-js";
import type { Lang } from "../lib/types";

function languageExtension(language: Lang) {
  switch (language) {
    case "html": {
      return html();
    }
    case "markdown": {
      return markdown();
    }
    case "plain": {
      return [];
    }
  }
}

type Props = { value: string; language: Lang; readonly?: boolean; onInput?: (value: string) => void };

export function CodeEditor(props: Props) {
  const [host, setHost] = createSignal<HTMLDivElement>();
  const [wordWrap, setWordWrap] = createSignal(true);
  const [copyStatus, setCopyStatus] = createSignal<"idle" | "copied" | "failed">("idle");
  const wordWrapCompartment = new Compartment();
  let view: EditorView | undefined;
  let copyStatusTimer: number | undefined;

  const wordWrapExtension = () => wordWrap() ? EditorView.lineWrapping : [];

  const resetCopyStatus = () => {
    if (copyStatusTimer) globalThis.clearTimeout(copyStatusTimer);
    copyStatusTimer = globalThis.setTimeout(() => setCopyStatus("idle"), 1600);
  };

  const copyValue = async () => {
    const value = view?.state.doc.toString() ?? props.value;
    try {
      await globalThis.navigator.clipboard.writeText(value);
      setCopyStatus("copied");
    } catch {
      setCopyStatus("failed");
    }
    resetCopyStatus();
  };

  createEffect(() => {
    const parent = host();
    if (view || !parent) return;

    view = new EditorView({
      parent,
      state: EditorState.create({
        doc: props.value,
        extensions: [
          lineNumbers(),
          keymap.of([indentWithTab]),
          languageExtension(props.language),
          catppuccinLatte,
          wordWrapCompartment.of(wordWrapExtension()),
          EditorView.editable.of(!props.readonly),
          EditorState.readOnly.of(Boolean(props.readonly)),
          EditorView.updateListener.of((update) => {
            if (update.docChanged && props.onInput) {
              props.onInput(update.state.doc.toString());
            }
          }),
        ],
      }),
    });
  });

  createEffect(() => {
    if (!view) return;
    view.dispatch({ effects: wordWrapCompartment.reconfigure(wordWrapExtension()) });
  });

  createEffect(() => {
    if (!view) return;
    const next = props.value;
    const current = view.state.doc.toString();
    if (next !== current) {
      view.dispatch({ changes: { from: 0, to: current.length, insert: next } });
    }
  });

  onCleanup(() => {
    if (copyStatusTimer) globalThis.clearTimeout(copyStatusTimer);
    view?.destroy();
  });

  return (
    <div class="code-editor">
      <div class="code-editor__toolbar" aria-label="Editor controls">
        <div class="editor-toggle" role="group" aria-label="Word wrap">
          <button
            type="button"
            classList={{ "is-active": wordWrap() }}
            aria-pressed={wordWrap()}
            onClick={() => setWordWrap(true)}>
            Wrap
          </button>
          <button
            type="button"
            classList={{ "is-active": !wordWrap() }}
            aria-pressed={!wordWrap()}
            onClick={() => setWordWrap(false)}>
            Clip
          </button>
        </div>
        <button
          type="button"
          class="editor-copy-button"
          aria-label={copyStatus() === "copied" ? "Copied editor contents" : "Copy editor contents"}
          title={copyStatus() === "failed" ? "Copy failed" : "Copy editor contents"}
          onClick={() => void copyValue()}>
          <span class="editor-copy-button__icon" aria-hidden="true" />
          <span aria-live="polite">{copyStatus() === "copied" ? "Copied" : "Copy"}</span>
        </button>
      </div>
      <div ref={setHost} class="code-editor__surface" />
    </div>
  );
}
