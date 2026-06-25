import { defaultOptions, type LayoutMode } from "$lib/workbench/store";
import { For } from "solid-js";
import { createStore } from "solid-js/store";
import type { PipelineOptions } from "../lib/types";
import { PageShell } from "./PageShell";
import { WorkbenchTabs } from "./WorkbenchTabs";

type Settings = { options: PipelineOptions; layout: LayoutMode };

const defaultSettings: Settings = { options: defaultOptions, layout: "split" };

const layoutLabels: Array<{ value: LayoutMode; label: string; hint: string }> = [
  { value: "split", label: "Split", hint: "Input and output side by side." },
  { value: "wide-output", label: "Wide output", hint: "Give the result more room." },
  { value: "input-collapsed", label: "Collapse input", hint: "Hide the editor after converting." },
];

type OptionFieldProps = {
  options: PipelineOptions;
  onUpdate: <Key extends keyof PipelineOptions>(key: Key, value: PipelineOptions[Key]) => void;
};

function OptionsFields(props: OptionFieldProps) {
  const update = <Key extends keyof PipelineOptions>(key: Key, value: PipelineOptions[Key]) =>
    props.onUpdate(key, value);

  return (
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
  );
}

function LayoutPicker(props: { value: LayoutMode; onChange: (layout: LayoutMode) => void }) {
  return (
    <div class="settings-layout" role="radiogroup" aria-label="Default layout">
      <For each={layoutLabels}>
        {(entry) => (
          <label classList={{ "is-active": props.value === entry.value }}>
            <input
              type="radio"
              name="default-layout"
              value={entry.value}
              checked={props.value === entry.value}
              onChange={() => props.onChange(entry.value)} />
            <span>
              <strong>{entry.label}</strong>
              <em>{entry.hint}</em>
            </span>
          </label>
        )}
      </For>
    </div>
  );
}

function SectionHeader(props: { title: string; description: string }) {
  return (
    <header class="settings-section__header">
      <h2>{props.title}</h2>
      <p>{props.description}</p>
    </header>
  );
}

export function SettingsPage() {
  const [settings, setSettings] = createStore<Settings>(structuredClone(defaultSettings));
  const dirty = () => JSON.stringify(settings) !== JSON.stringify(defaultSettings);

  const updateOption = <Key extends keyof PipelineOptions>(key: Key, value: PipelineOptions[Key]) =>
    setSettings("options", key, value);

  const save = () => {
    console.info("save settings", settings);
  };

  const reset = () => setSettings(structuredClone(defaultSettings));

  return (
    <PageShell eyebrow="Workbench" title="Settings" headerBefore={<WorkbenchTabs />} variant="workbench">
      <form class="settings-page" onSubmit={(event) => event.preventDefault()}>
        <p class="settings-page__intro">
          These defaults apply to new workbench sessions. Changes here do not alter runs you have already saved.
        </p>

        <section class="settings-section">
          <SectionHeader
            title="Extraction defaults"
            description="Starting values for the advanced options panel in the workbench." />
          <OptionsFields options={settings.options} onUpdate={updateOption} />
        </section>

        <section class="settings-section">
          <SectionHeader title="Default layout" description="How the workbench arranges the input and output panes." />
          <LayoutPicker value={settings.layout} onChange={(layout) => setSettings("layout", layout)} />
        </section>

        <div class="settings-actions">
          <button type="submit" class="button button--primary" disabled={!dirty()} onClick={save}>Save defaults</button>
          <button type="button" class="button button--secondary" disabled={!dirty()} onClick={reset}>Reset</button>
        </div>
      </form>
    </PageShell>
  );
}
