/* @refresh reload */
import { SettingsProvider } from "$lib/settings/context";
import { defaultSettings, loadSettings } from "$lib/settings/store";
import { setupI18n } from "@lingui/core";
import { I18nProvider } from "@lingui/solid";
import { render } from "solid-js/web";
import "virtual:uno.css";
import "$styles/style.css";
import App from "./App.tsx";
import { defaultLocale, loadCatalog } from "./i18n";

const root = document.getElementById("root");
const i18n = setupI18n();

/**
 * Here we load i18n and persisted settings together before first render so the
 * workbench starts from the user's saved defaults with no flash of defaults.
 */
const settings = await loadSettings().catch(() => defaultSettings);

await loadCatalog(defaultLocale, i18n);

render(() => (
  <I18nProvider i18n={i18n}>
    <SettingsProvider initial={settings}>
      <App />
    </SettingsProvider>
  </I18nProvider>
), root!);
