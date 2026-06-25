import { lingui } from "@lingui/vite-plugin";
import unocss from "@unocss/vite";
import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

export default defineConfig({
  plugins: [unocss(), solid({ babel: { plugins: ["@lingui/babel-plugin-lingui-macro"] } }), lingui()],
  server: { open: true },
  resolve: {
    alias: { $components: "/src/components", $lib: "/src/lib", $pages: "/src/pages", $styles: "/src/styles" },
  },
});
