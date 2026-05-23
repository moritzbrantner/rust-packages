import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  optimizeDeps: {
    exclude: ["@mb-rust/video-analysis-ingest-wasm"],
  },
  plugins: [react()],
});
