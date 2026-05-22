import path from "node:path";
import { fileURLToPath } from "node:url";

const dirname = path.dirname(fileURLToPath(import.meta.url));
const textLinguisticsWasmEntry = path.resolve(dirname, "../text-linguistics-wasm/index.js");

export default {
  optimizeDeps: {
    exclude: ["@mb-rust/text-linguistics-wasm"],
  },
  resolve: {
    alias: {
      "@mb-rust/text-linguistics-wasm": textLinguisticsWasmEntry,
      "react/jsx-runtime": path.resolve(
        dirname,
        "../../node_modules/.bun/node_modules/react/jsx-runtime.js",
      ),
      "react/jsx-dev-runtime": path.resolve(
        dirname,
        "../../node_modules/.bun/node_modules/react/jsx-dev-runtime.js",
      ),
      "react-dom/client": path.resolve(
        dirname,
        "../../node_modules/.bun/node_modules/react-dom/client.js",
      ),
      react: path.resolve(dirname, "../../node_modules/.bun/node_modules/react/index.js"),
    },
  },
};
