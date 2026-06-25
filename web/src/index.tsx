/* @refresh reload */
import { setupI18n } from "@lingui/core";
import { I18nProvider } from "@lingui/solid";
import { render } from "solid-js/web";
import "virtual:uno.css";
import "$styles/style.css";
import App from "./App.tsx";
import { defaultLocale, loadCatalog } from "./i18n";

const root = document.getElementById("root");
const i18n = setupI18n();

await loadCatalog(defaultLocale, i18n);

render(() => (
  <I18nProvider i18n={i18n}>
    <App />
  </I18nProvider>
), root!);
