import presetIcons from "@unocss/preset-icons";
import { defineConfig } from "@unocss/vite";

export default defineConfig({
  presets: [presetIcons({ extraProperties: { display: "inline-block", "vertical-align": "middle" } })],
});
