import { lingui } from "@lingui/vite-plugin";
import unocss from "@unocss/vite";
import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

/**
 * ## Uno
 *
 * Icons only (i-ph-* for phosphor)
 *
 * ## Proxy
 *
 * Forward same-origin `/api` calls to the local API service in dev.
 *
 * Production will use Cloudflare, which strips the `/api` prefix before
 * forwarding, so the rewrite keeps request paths consistent across environments.
 */
const conf: ReturnType<typeof defineConfig> = {
  plugins: [unocss(), solid({ babel: { plugins: ["@lingui/babel-plugin-lingui-macro"] } }), lingui()],
  server: {
    proxy: {
      "/api": {
        target: "http://localhost:3000",
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/api/, ""),
      },
    },
  },
  resolve: {
    alias: { $components: "/src/components", $lib: "/src/lib", $pages: "/src/pages", $styles: "/src/styles" },
  },
};

export default defineConfig(conf);
