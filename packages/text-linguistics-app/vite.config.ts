import path from "node:path";
import { fileURLToPath } from "node:url";

const dirname = path.dirname(fileURLToPath(import.meta.url));

export default {
  resolve: {
    alias: {
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
