import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// GitHub Pages serves a project site from /<repo>/, so the base path is set
// from the BASE_PATH env var at build time (e.g. "/lumenscan-web/"). Locally
// and on a root deploy it stays "/".
export default defineConfig({
  base: process.env.BASE_PATH ?? "/",
  plugins: [react()],
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
