import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  optimizeDeps: {
    exclude: ["@mb-rust/three-d-processing-mesh-wasm"],
  },
  plugins: [react()],
});
