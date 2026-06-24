# Web To-Do

The web app has two primary flows:

- Convert pasted HTML markup in the browser through WASM.
- Use the API docs for server-side URL extraction.

## Pages

- `/`: landing page with a direct workbench CTA, before/after extraction
  diagram, capability summary, and links to the API and samples.
- `/workbench`: the main extraction workspace.
- `/workbench/runs`: saved local runs, with source, status, elapsed time,
  length, title, timestamp, and options.
- `/workbench/runs/:id`: one saved extraction result, with output, metadata,
  diagnostics, and options.
- `/workbench/samples`: a browsable gallery of curated fixtures and known edge
  cases.
- `/api`: Markdown-rendered API docs and reference examples.
- `/workbench/settings`: output preferences and history settings.

## Controls

- Add a searchable sample picker for pasted HTML fixtures.
- Add true resizable split panes.
- Keep failed input intact and show field-level recovery messages.

## Implementation Order

- [ ] Back run history with Dexie.js.
- [ ] Add the sample gallery.
- [ ] Add `/workbench/settings` persistence.
