import { indentWithTab } from "@codemirror/commands";
import { html } from "@codemirror/lang-html";
import { markdown } from "@codemirror/lang-markdown";
import { EditorState } from "@codemirror/state";
import { EditorView, keymap, lineNumbers } from "@codemirror/view";
import { createEffect, createSignal, onCleanup } from "solid-js";

type Props = {
  value: string;
  language: "html" | "markdown" | "plain";
  readonly?: boolean;
  onInput?: (value: string) => void;
};

// TODO: add catppuccin latte theme
export function CodeEditor(props: Props) {
  const [host, setHost] = createSignal<HTMLDivElement>();
  let view: EditorView | undefined;

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
          EditorView.lineWrapping,
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
    const next = props.value;
    const current = view.state.doc.toString();
    if (next !== current) {
      view.dispatch({ changes: { from: 0, to: current.length, insert: next } });
    }
  });

  onCleanup(() => view?.destroy());

  return <div ref={setHost} class="code-editor" />;
}

function languageExtension(language: Props["language"]) {
  if (language === "html") return html();
  if (language === "markdown") return markdown();
  return [];
}
