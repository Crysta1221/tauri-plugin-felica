import { defineConfig, type OutExtensionContext } from "tsdown";

export default defineConfig({
  entry: {
    index: "src/typescript/index.ts",
    labels: "src/typescript/labels/index.ts",
  },
  outDir: "dist-js",
  format: ["esm", "cjs"],
  dts: {
    generator: "tsgo",
  },
  clean: true,
  platform: "neutral",
  outExtensions({ format }: OutExtensionContext) {
    return {
      js: format === "cjs" ? ".cjs" : ".js",
    };
  },
  deps: {
    neverBundle: [/^@tauri-apps\/api/u],
  },
});
