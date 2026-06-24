import type { PipelineOptions } from "../lib/types";

type OptionsPanelProps = { options: PipelineOptions; onChangeOpts: (options: PipelineOptions) => void };

const presets: Array<{ id: string; label: string; options: Partial<PipelineOptions> }> = [
  { id: "default", label: "Default", options: { contentSelector: "", charThreshold: 0, keepClasses: false, diagnostics: false } },
  { id: "strict", label: "Strict article", options: { contentSelector: "article, main", charThreshold: 1_200, diagnostics: false } },
  { id: "media", label: "Keep media", options: { contentSelector: "", charThreshold: 0, keepClasses: false, diagnostics: false } },
  { id: "debug", label: "Debug", options: { diagnostics: true, charThreshold: 0 } },
  { id: "styling", label: "Preserve styling", options: { keepClasses: true, charThreshold: 0 } },
];

export function OptionsPanel(props: OptionsPanelProps) {
  function update<Key extends keyof PipelineOptions>(key: Key, value: PipelineOptions[Key]) {
    props.onChangeOpts({ ...props.options, [key]: value });
  }

  return (
    <section class="options-panel" aria-label="Extraction options">
      <div class="preset-row" role="group" aria-label="Option presets">
        {presets.map((preset) => (
          <button type="button" onClick={() => props.onChangeOpts({ ...props.options, ...preset.options })}>
            {preset.label}
          </button>
        ))}
      </div>

      <div class="option-groups">
        <fieldset>
          <legend>Extraction</legend>
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
        </fieldset>

        <fieldset>
          <legend>Media</legend>
          <p>Images and embedded content are kept when the extractor accepts them as part of the article.</p>
        </fieldset>

        <fieldset>
          <legend>Metadata</legend>
          <label>
            <span>Base URL</span>
            <input
              type="url"
              value={props.options.baseUrl}
              placeholder="https://example.com"
              onInput={(event) => update("baseUrl", event.currentTarget.value)} />
          </label>
        </fieldset>

        <fieldset>
          <legend>Styling</legend>
          <label class="check-option">
            <input
              type="checkbox"
              checked={props.options.keepClasses}
              onInput={(event) => update("keepClasses", event.currentTarget.checked)} />
            <span>Keep classes</span>
          </label>
        </fieldset>

        <fieldset>
          <legend>Site rules</legend>
          <p>Built-in site cleanup rules are applied automatically when available.</p>
        </fieldset>

        <fieldset>
          <legend>Debug</legend>
          <label class="check-option">
            <input
              type="checkbox"
              checked={props.options.diagnostics}
              onInput={(event) => update("diagnostics", event.currentTarget.checked)} />
            <span>Diagnostics</span>
          </label>
        </fieldset>
      </div>
    </section>
  );
}
