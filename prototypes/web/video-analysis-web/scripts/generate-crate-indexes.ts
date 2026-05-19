import { copyFile, mkdir, readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

import { slugifyPackageName, type WorkspaceArchitectureResponse } from "../src/workspaceArchitecture";

const projectRoot = fileURLToPath(new URL("..", import.meta.url));
const distDir = `${projectRoot}/dist`;
const indexPath = `${distDir}/index.html`;
const dataPath = `${projectRoot}/public/workspace-architecture.json`;

const architecture = JSON.parse(await readFile(dataPath, "utf8")) as WorkspaceArchitectureResponse;

await writeFile(`${distDir}/.nojekyll`, "", "utf8");
await copyFile(indexPath, `${distDir}/404.html`);

for (const pkg of architecture.packages) {
  const crateDir = `${distDir}/crates/${slugifyPackageName(pkg.name)}`;
  await mkdir(crateDir, { recursive: true });
  await copyFile(indexPath, `${crateDir}/index.html`);
}

console.log(`generated ${architecture.packages.length} crate pages`);
