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

const servicePackages = architecture.packages.filter((pkg) => pkg.kind === "rust" && pkg.name.endsWith("-server"));

for (const service of servicePackages) {
  const serviceDir = `${distDir}/wrappers/${slugifyPackageName(service.name.replace(/-server$/, ""))}`;
  await mkdir(serviceDir, { recursive: true });
  await copyFile(indexPath, `${serviceDir}/index.html`);
}

console.log(`generated ${architecture.packages.length} crate pages and ${servicePackages.length} wrapper pages`);
