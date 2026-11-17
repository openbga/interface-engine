import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// The Tauri window loads the dev server in development and `dist` in builds
// (src-tauri/tauri.conf.json). Vitest settings live here too. The `@/` alias comes
// from tsconfig.json.
export default defineConfig({
  plugins: [react()],
  resolve: { tsconfigPaths: true },
  clearScreen: false,
  server: {
    port: 5174,
    strictPort: true,
    // Files outside console/: the preset catalog (crates/packie-config/profiles) and the
    // What's new entries (changelog.d).
    fs: { allow: [".."] },
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
  test: {
    fsModuleCache: false,
    environment: "jsdom",
    testTimeout: 20_000,
    include: ["src/**/*.test.{ts,tsx}", "scripts/**/*.test.mjs"],
    globalSetup: ["src/lib/test/globalSetup.ts"],
    setupFiles: ["src/lib/test/setup.ts"],
    restoreMocks: true,
  },
});
