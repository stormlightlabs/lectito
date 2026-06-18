import type { AppMode, PipelineOptions } from "../lib/types";

type OptionsPanelProps = { mode: AppMode; options: PipelineOptions; onChangeOpts: (options: PipelineOptions) => void };

export function OptionsPanel(props: OptionsPanelProps) {
  function update<Key extends keyof PipelineOptions>(key: Key, value: PipelineOptions[Key]) {
    props.onChangeOpts({ ...props.options, [key]: value });
  }

  return (
    <section class="options-panel" aria-label="Extraction options">
      <div class="field-grid">
        <label>
          <span>Base URL</span>
          <input
            type="url"
            value={props.options.baseUrl}
            disabled={props.mode === "url"}
            placeholder="https://example.com"
            onInput={(event) => update("baseUrl", event.currentTarget.value)} />
        </label>
        <label>
          <span>Content selector</span>
          <input
            value={props.options.contentSelector}
            placeholder="main article"
            onInput={(event) => update("contentSelector", event.currentTarget.value)} />
        </label>
        <label>
          <span>Character threshold</span>
          <input
            type="number"
            min="0"
            step="50"
            value={props.options.charThreshold}
            onInput={(event) => update("charThreshold", event.currentTarget.valueAsNumber || 0)} />
        </label>
      </div>
      <div class="check-row">
        <label>
          <input
            type="checkbox"
            checked={props.options.keepClasses}
            onInput={(event) => update("keepClasses", event.currentTarget.checked)} />
          <span>Keep classes</span>
        </label>
        <label>
          <input
            type="checkbox"
            checked={props.options.diagnostics}
            onInput={(event) => update("diagnostics", event.currentTarget.checked)} />
          <span>Diagnostics</span>
        </label>
      </div>
    </section>
  );
}
