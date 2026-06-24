import { lingui } from "@lingui/vite-plugin";
import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

export default defineConfig({
  plugins: [solid({ babel: { plugins: ["@lingui/babel-plugin-lingui-macro"] } }), lingui()],
  resolve: {
    alias: { $components: "/src/components", $lib: "/src/lib", $pages: "/src/pages", $styles: "/src/styles" },
  },
});
