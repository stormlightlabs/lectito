import { defineConfig } from "@lingui/solid/config";

export default defineConfig({
  sourceLocale: "en",
  locales: ["en"],
  catalogs: [{ path: "src/locales/{locale}", include: ["src"] }],
});
