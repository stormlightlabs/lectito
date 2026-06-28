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
- `/api-docs`: Markdown-rendered API docs and reference examples.
- `/workbench/settings`: output preferences and history settings.

## Controls

- Add a searchable sample picker for pasted HTML fixtures.
- Add true resizable split panes.
- Keep failed input intact and show field-level recovery messages.

## Implementation Order

- [x] Back run history with Dexie.js.
- [x] Add the sample gallery.
- [x] Add `/workbench/settings` persistence.
- [x] Add a URL input mode to the workbench and wire it to `extractUrlWithApi`.
  - The landing page already says users can enter a URL, but the workbench only
    exposes pasted HTML.
  - Keep pasted HTML out of the URL. Store only lightweight mode state in search
    params.
  - Save URL runs with a URL-based source label instead of `Pasted HTML`.
- [x] Add API error states that users can act on:
  - `429` should show that the request was rate-limited and, when present, use
    `Retry-After`.
  - Timeout and network failures should distinguish API unavailability from
    extraction failures.
  - Keep the raw structured error available in diagnostics when diagnostics are
    enabled.
- [x] Confirm the web app uses `/api` in production and does not expose
      `https://lectito-api.stormlightlabs.org` in public UI, examples, or docs.
- [x] Update the in-app API page after the public proxy is live:
  - Use `https://lectito.stormlightlabs.org/api/v1/...` examples.
  - Keep endpoint names aligned with `/v1/extract`, `/v1/evaluate`, and
    `/v1/transform`.
  - Remove or correct any documented Markdown options that `/v1/transform` does
    not actually honor.
- [ ] Add local data controls:
  - Delete one saved run.
  - Clear all saved runs.
  - Clear local settings and history from the Settings page.
- [ ] Add a way to reopen a saved run in the workbench.
- [ ] Add export/import for saved runs, or state clearly that runs are local and
      disposable.
- [ ] Add a small API status indicator or connection check for URL mode.
- [ ] Run a responsive and accessibility smoke test on:
  - `/`
  - `/workbench`
  - `/workbench/samples`
  - `/workbench/runs`
  - `/workbench/settings`
- [ ] Run the i18n extraction and compile commands after copy changes:

  ```sh
  pnpm --dir packages/web run messages:extract
  pnpm --dir packages/web run messages:compile
  ```
