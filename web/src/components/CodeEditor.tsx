import { createEffect, createMemo, createSignal, onCleanup, Show } from "solid-js";
import type { Lang } from "../lib/types";
import { Icon } from "./Icon";

type CodeMirrorModules = Awaited<ReturnType<typeof loadCodeMirror>>;

async function loadCodeMirror() {
  const [
    { daOnePaperLightCodeMirrorTheme },
    { indentWithTab },
    { html },
    { markdown },
    { Compartment, EditorState },
    { EditorView, keymap, lineNumbers },
  ] = await Promise.all([
    import("$lib/codemirror-theme"),
    import("@codemirror/commands"),
    import("@codemirror/lang-html"),
    import("@codemirror/lang-markdown"),
    import("@codemirror/state"),
    import("@codemirror/view"),
  ]);

  return {
    daOnePaperLightCodeMirrorTheme,
    indentWithTab,
    html,
    markdown,
    Compartment,
    EditorState,
    EditorView,
    keymap,
    lineNumbers,
  };
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

export type CodeEditorProps = {
  value: string;
  language: Lang;
  readonly?: boolean;
  statusText?: string;
  onInput?: (value: string) => void;
};

type EditorStatus = {
  bytes: number;
  chars: number;
  column: number;
  language: Lang;
  line: number;
  lines: number;
  readonly: boolean;
  selected: number;
};

type EditorActionsMenuProps = {
  copied: boolean;
  copyFailed: boolean;
  wordWrap: boolean;
  onClip: () => void;
  onCopy: () => void;
  onWrap: () => void;
};

function EditorActionsMenu(props: EditorActionsMenuProps) {
  return (
    <details class="overflow-menu overflow-menu--up">
      <summary class="button button--secondary button--icon" aria-label="Editor actions" title="Editor actions">
        <Icon kind="more" />
      </summary>
      <div class="overflow-menu__panel">
        <button
          type="button"
          classList={{ "is-active": props.wordWrap }}
          aria-pressed={props.wordWrap}
          onClick={props.onWrap}>
          Wrap
        </button>
        <button
          type="button"
          classList={{ "is-active": !props.wordWrap }}
          aria-pressed={!props.wordWrap}
          onClick={props.onClip}>
          Clip
        </button>
        <button
          type="button"
          aria-label={props.copied ? "Copied editor contents" : "Copy editor contents"}
          title={props.copyFailed ? "Copy failed" : "Copy editor contents"}
          onClick={props.onCopy}>
          {props.copied ? "Copied" : "Copy"}
        </button>
      </div>
    </details>
  );
}

function byteSize(value: string) {
  return new Blob([value]).size;
}

function sizeLabel(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

function languageLabel(language: Lang) {
  if (language === "html") return "HTML";
  if (language === "markdown") return "Markdown";
  return "Plain text";
}

function initialStatus(props: CodeEditorProps): EditorStatus {
  return {
    bytes: byteSize(props.value),
    chars: props.value.length,
    column: 1,
    language: props.language,
    line: 1,
    lines: props.value.split(/\r\n|\r|\n/).length,
    readonly: Boolean(props.readonly),
    selected: 0,
  };
}

export function CodeEditor(props: CodeEditorProps) {
  const [host, setHost] = createSignal<HTMLDivElement>();
  const [wordWrap, setWordWrap] = createSignal(true);
  const [copyStatus, setCopyStatus] = createSignal<"idle" | "copied" | "failed">("idle");
  const [status, setStatus] = createSignal<EditorStatus>(initialStatus(props));
  let editorViewModule: CodeMirrorModules["EditorView"] | undefined;
  let wordWrapCompartment: InstanceType<CodeMirrorModules["Compartment"]> | undefined;
  let view: InstanceType<CodeMirrorModules["EditorView"]> | undefined;
  let copyStatusTimer: number | undefined;
  let disposed = false;

  const wordWrapExtension = () => wordWrap() && editorViewModule ? editorViewModule.lineWrapping : [];
  const rightStatusItems = createMemo(() => {
    const current = status();
    return [
      languageLabel(current.language),
      current.readonly ? "Read-only" : "Editable",
      wordWrap() ? "Wrap On" : "Wrap Off",
      sizeLabel(current.bytes),
      `${current.chars.toLocaleString()}ch`,
      `${current.lines.toLocaleString()}Ln`,
      `${current.line.toLocaleString()}:${current.column.toLocaleString()}`,
      current.selected > 0 ? `${current.selected.toLocaleString()} selected` : "",
    ].filter(Boolean);
  });

  const updateStatus = () => {
    if (!view) {
      setStatus(initialStatus(props));
      return;
    }

    const state = view.state;
    const head = state.selection.main.head;
    const line = state.doc.lineAt(head);
    const selected = state.selection.ranges.reduce((total, range) => total + Math.abs(range.to - range.from), 0);

    setStatus({
      bytes: byteSize(state.doc.toString()),
      chars: state.doc.length,
      column: head - line.from + 1,
      language: props.language,
      line: line.number,
      lines: state.doc.lines,
      readonly: Boolean(props.readonly),
      selected,
    });
  };

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
            modules.daOnePaperLightCodeMirrorTheme,
            wordWrapCompartment.of(wordWrapExtension()),
            modules.EditorView.editable.of(!props.readonly),
            modules.EditorState.readOnly.of(Boolean(props.readonly)),
            modules.EditorView.updateListener.of((update) => {
              if (update.docChanged && props.onInput) {
                props.onInput(update.state.doc.toString());
              }
              if (update.docChanged || update.selectionSet) {
                updateStatus();
              }
            }),
          ],
        }),
      });
      updateStatus();
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
    } else {
      updateStatus();
    }
  });

  onCleanup(() => {
    disposed = true;
    if (copyStatusTimer) globalThis.clearTimeout(copyStatusTimer);
    view?.destroy();
  });

  return (
    <div class="code-editor">
      <div ref={setHost} class="code-editor__surface" />
      <div class="code-editor__status" aria-label="Editor status">
        <Show when={props.statusText}>
          <strong>{props.statusText}</strong>
        </Show>
        <div style={{ display: "flex", flex: 1, "align-items": "center", "justify-content": "end" }}>
          <Show when={rightStatusItems().length > 0}>
            <span>{rightStatusItems().join(" / ")}</span>
          </Show>
          <EditorActionsMenu
            copied={copyStatus() === "copied"}
            copyFailed={copyStatus() === "failed"}
            wordWrap={wordWrap()}
            onClip={() => setWordWrap(false)}
            onCopy={() => void copyValue()}
            onWrap={() => setWordWrap(true)} />
        </div>
      </div>
    </div>
  );
}
