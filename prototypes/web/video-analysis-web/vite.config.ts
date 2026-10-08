import react from "@vitejs/plugin-react";
import { readdirSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { defineConfig, type Plugin } from "vite";

import { loadWorkspaceArchitecture } from "./workspaceArchitectureLoader";

const workspaceRoot = fileURLToPath(new URL("../../..", import.meta.url));
const packagesRoot = fileURLToPath(new URL("../../../packages", import.meta.url));
const uiSourceRoot = fileURLToPath(new URL("../../../packages/video-analysis-ui/src", import.meta.url));
const textCoreWasmEntry = fileURLToPath(
  new URL("../../../packages/text-core-wasm/index.js", import.meta.url),
);
const textLinguisticsWasmEntry = fileURLToPath(
  new URL("../../../packages/text-linguistics-wasm/index.js", import.meta.url),
);
const require = createRequire(import.meta.url);

export default defineConfig({
  base: process.env.PAGES_BASE_PATH ?? "/",
  plugins: [react(), workspaceArchitectureApi()],
  optimizeDeps: {
    exclude: ["@moritzbrantner/text-core-wasm", "@moritzbrantner/text-linguistics-wasm"],
  },
  resolve: {
    alias: [
      { find: /^@moritzbrantner\/text-core-wasm$/, replacement: textCoreWasmEntry },
      { find: /^@moritzbrantner\/text-linguistics-wasm$/, replacement: textLinguisticsWasmEntry },
      ...workspaceWasmAliases(),
      { find: /^@moritzbrantner\/video-analysis-ui$/, replacement: `${uiSourceRoot}/index.ts` },
      {
        find: /^@moritzbrantner\/video-analysis-ui\/tailwind-content$/,
        replacement: `${uiSourceRoot}/tailwind-content.ts`,
      },
      { find: /^@moritzbrantner\/video-analysis-ui\/([^/]+)$/, replacement: `${uiSourceRoot}/$1/index.tsx` },
      { find: /^react\/jsx-runtime$/, replacement: require.resolve("react/jsx-runtime") },
      { find: /^react\/jsx-dev-runtime$/, replacement: require.resolve("react/jsx-dev-runtime") },
      { find: /^react-dom\/client$/, replacement: require.resolve("react-dom/client") },
      { find: /^react$/, replacement: require.resolve("react") },
    ],
  },
});

function workspaceWasmAliases() {
  return readdirSync(packagesRoot, { withFileTypes: true })
    .filter((entry) => entry.isDirectory() && entry.name.endsWith("-wasm"))
    .flatMap((entry) => {
      const packageJsonPath = `${packagesRoot}/${entry.name}/package.json`;
      const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf8")) as { name?: string };
      if (!packageJson.name) {
        return [];
      }
      return [
        {
          find: new RegExp(`^${escapeRegExp(packageJson.name)}$`),
          replacement: `${packagesRoot}/${entry.name}/index.js`,
        },
      ];
    });
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

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
