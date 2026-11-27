import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// `grace serve --port 4319 --allow-origin http://localhost:5173` runs the API for `pnpm dev`.
const api = process.env.GRACE_API ?? "http://127.0.0.1:4319";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  // Relative asset paths work both in the desktop app and under `grace serve`.
  base: "./",
  server: {
    port: 5173,
    strictPort: true,
    proxy: { "/api": { target: api, changeOrigin: true } },
  },
  build: {
    target: "es2022",
    chunkSizeWarningLimit: 800,
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
    css: false,
  },
});
