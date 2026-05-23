import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  optimizeDeps: {
    exclude: ["@mb-rust/image-analysis-models-wasm"],
  },
  plugins: [react()],
});
