import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  optimizeDeps: {
    exclude: ["@mb-rust/data-inversion-core-wasm"],
  },
  plugins: [react()],
});
