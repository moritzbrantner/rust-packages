import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";
import { defineConfig, type Plugin } from "vite";

import { loadWorkspaceArchitecture } from "./workspaceArchitectureLoader";

const workspaceRoot = fileURLToPath(new URL("../../..", import.meta.url));
const uiSourceRoot = fileURLToPath(new URL("../../../packages/video-analysis-ui/src", import.meta.url));
const textCoreWasmEntry = fileURLToPath(
  new URL("../../../packages/text-core-wasm/index.js", import.meta.url),
);
const rootNodeModules = fileURLToPath(new URL("../../../node_modules/.bun/node_modules/", import.meta.url));

export default defineConfig({
  base: process.env.PAGES_BASE_PATH ?? "/",
  plugins: [react(), workspaceArchitectureApi()],
  optimizeDeps: {
    exclude: ["@mb-rust/text-core-wasm"],
  },
  resolve: {
    alias: [
      { find: /^@mb-rust\/text-core-wasm$/, replacement: textCoreWasmEntry },
      { find: /^@video-analysis\/ui$/, replacement: `${uiSourceRoot}/index.ts` },
      { find: /^@video-analysis\/ui\/tailwind-content$/, replacement: `${uiSourceRoot}/tailwind-content.ts` },
      { find: /^@video-analysis\/ui\/([^/]+)$/, replacement: `${uiSourceRoot}/$1/index.tsx` },
      { find: /^react\/jsx-runtime$/, replacement: `${rootNodeModules}react/jsx-runtime.js` },
      { find: /^react\/jsx-dev-runtime$/, replacement: `${rootNodeModules}react/jsx-dev-runtime.js` },
      { find: /^react-dom\/client$/, replacement: `${rootNodeModules}react-dom/client.js` },
      { find: /^react$/, replacement: `${rootNodeModules}react/index.js` },
    ],
  },
});

function workspaceArchitectureApi(): Plugin {
  return {
    name: "workspace-architecture-api",
    configureServer(server) {
      server.middlewares.use("/api/workspace-architecture", handleWorkspaceArchitecture);
      server.middlewares.use("/api/packages", handlePackages);
    },
    configurePreviewServer(server) {
      server.middlewares.use("/api/workspace-architecture", handleWorkspaceArchitecture);
      server.middlewares.use("/api/packages", handlePackages);
    },
  };
}

async function handleWorkspaceArchitecture(req: any, res: any, next: any) {
  if (req.method !== "GET") {
    next();
    return;
  }

  try {
    sendJson(res, 200, await loadWorkspaceArchitecture(workspaceRoot));
  } catch (error) {
    sendJson(res, 500, {
      message: error instanceof Error ? error.message : String(error),
    });
  }
}

async function handlePackages(req: any, res: any, next: any) {
  if (req.method !== "GET") {
    next();
    return;
  }

  try {
    const url = new URL(req.url ?? "", "http://localhost");
    const name = url.searchParams.get("name")?.trim();
    const architecture = await loadWorkspaceArchitecture(workspaceRoot);
    if (!name) {
      sendJson(res, 200, architecture.packages);
      return;
    }

    const packageInfo = architecture.packages.find((pkg) => pkg.name === name);
    if (!packageInfo) {
      sendJson(res, 404, { message: `unknown package \`${name}\`` });
      return;
    }

    sendJson(res, 200, packageInfo);
  } catch (error) {
    sendJson(res, 500, {
      message: error instanceof Error ? error.message : String(error),
    });
  }
}

function sendJson(res: any, status: number, payload: unknown) {
  res.statusCode = status;
  res.setHeader("Content-Type", "application/json");
  res.end(JSON.stringify(payload));
}
