import { createEffect, createSignal, onCleanup } from "solid-js";
import type { Lang } from "../lib/types";

type CodeMirrorModules = Awaited<ReturnType<typeof loadCodeMirror>>;

async function loadCodeMirror() {
  const [
    { catppuccinLatte },
    { indentWithTab },
    { html },
    { markdown },
    { Compartment, EditorState },
    { EditorView, keymap, lineNumbers },
  ] = await Promise.all([
    import("@catppuccin/codemirror"),
    import("@codemirror/commands"),
    import("@codemirror/lang-html"),
    import("@codemirror/lang-markdown"),
    import("@codemirror/state"),
    import("@codemirror/view"),
  ]);

  return { catppuccinLatte, indentWithTab, html, markdown, Compartment, EditorState, EditorView, keymap, lineNumbers };
}

function languageExtension(language: Lang, modules: CodeMirrorModules) {
  switch (language) {
    case "html": {
      return modules.html();
    }
    case "markdown": {
      return modules.markdown();
    }
    case "plain": {
      return [];
    }
  }
}

export type CodeEditorProps = { value: string; language: Lang; readonly?: boolean; onInput?: (value: string) => void };

export function CodeEditor(props: CodeEditorProps) {
  const [host, setHost] = createSignal<HTMLDivElement>();
  const [wordWrap, setWordWrap] = createSignal(true);
  const [copyStatus, setCopyStatus] = createSignal<"idle" | "copied" | "failed">("idle");
  let editorViewModule: CodeMirrorModules["EditorView"] | undefined;
  let wordWrapCompartment: InstanceType<CodeMirrorModules["Compartment"]> | undefined;
  let view: InstanceType<CodeMirrorModules["EditorView"]> | undefined;
  let copyStatusTimer: number | undefined;
  let disposed = false;

  const wordWrapExtension = () => wordWrap() && editorViewModule ? editorViewModule.lineWrapping : [];

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

    void loadCodeMirror().then((modules) => {
      if (disposed || view) return;

      editorViewModule = modules.EditorView;
      wordWrapCompartment = new modules.Compartment();

      view = new modules.EditorView({
        parent,
        state: modules.EditorState.create({
          doc: props.value,
          extensions: [
            modules.lineNumbers(),
            modules.keymap.of([modules.indentWithTab]),
            languageExtension(props.language, modules),
            modules.catppuccinLatte,
            wordWrapCompartment.of(wordWrapExtension()),
            modules.EditorView.editable.of(!props.readonly),
            modules.EditorState.readOnly.of(Boolean(props.readonly)),
            modules.EditorView.updateListener.of((update) => {
              if (update.docChanged && props.onInput) {
                props.onInput(update.state.doc.toString());
              }
            }),
          ],
        }),
      });
    });
  });

  createEffect(() => {
    wordWrap();
    if (!view || !wordWrapCompartment) return;
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
    disposed = true;
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
