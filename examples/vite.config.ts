import path from "node:path";
import { defineConfig, lazyPlugins } from "vite-plus";
import react from "@vitejs/plugin-react";
import { tanstackRouter } from "@tanstack/router-plugin/vite";
import tailwindcss from "@tailwindcss/vite";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  resolve: {
    alias: {
      "@": path.resolve(import.meta.dirname, "./src"),
      "tauri-plugin-felica-api/labels": path.resolve(import.meta.dirname, "../dist-js/labels.js"),
      "tauri-plugin-felica-api": path.resolve(import.meta.dirname, "../dist-js/index.js"),
    },
  },
  fmt: {},
  lint: {
    ignorePatterns: ["dist/**", "src-tauri/**", "src/routeTree.gen.ts"],
    jsPlugins: [{ name: "vite-plus", specifier: "vite-plus/oxlint-plugin" }, "oxlint-tailwindcss"],
    settings: {
      tailwindcss: { entryPoint: "src/style.css" },
    },
    rules: {
      "vite-plus/prefer-vite-plus-imports": "error",
      "tailwindcss/no-unknown-classes": "error",
      "tailwindcss/no-conflicting-classes": "error",
      "tailwindcss/enforce-sort-order": "warn",
      // IntelliSense suggestCanonicalClasses for arbitrary lengths (min-h-[280px] → min-h-70).
      "tailwindcss/prefer-scale-token": ["warn", { step: 0.25 }],
    },
    options: { typeAware: true, typeCheck: true },
  },
  plugins: lazyPlugins(() => [
    tanstackRouter({ target: "react", autoCodeSplitting: true }),
    react(),
    tailwindcss(),
  ]),
  clearScreen: false,
  server: {
    host: host || false,
    port: 1420,
    strictPort: true,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
    fs: {
      allow: [path.resolve(import.meta.dirname, "..")],
    },
  },
});
