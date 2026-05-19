import { mkdir, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

import { loadWorkspaceArchitecture } from "../workspaceArchitectureLoader";

const workspaceRoot = fileURLToPath(new URL("../../../..", import.meta.url));
const publicDir = fileURLToPath(new URL("../public", import.meta.url));
const outputPath = `${publicDir}/workspace-architecture.json`;

const architecture = await loadWorkspaceArchitecture(workspaceRoot);

await mkdir(publicDir, { recursive: true });
await writeFile(outputPath, `${JSON.stringify(architecture, null, 2)}\n`, "utf8");

console.log(`wrote ${outputPath} with ${architecture.packages.length} packages`);
