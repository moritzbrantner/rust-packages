import { copyFile, mkdir, readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

import {
  packageDomainOrder,
  slugifyPackageName,
  type WorkspaceArchitectureResponse,
} from "../src/workspaceArchitecture";

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

const serviceDomains = new Set(servicePackages.map((pkg) => pkg.domain));
const categoryDomains = packageDomainOrder.filter((domain) => serviceDomains.has(domain));

for (const domain of categoryDomains) {
  const categoryDir = `${distDir}/categories/${domain}`;
  await mkdir(categoryDir, { recursive: true });
  await copyFile(indexPath, `${categoryDir}/index.html`);
}

console.log(
  `generated ${architecture.packages.length} crate pages, ${servicePackages.length} wrapper pages, and ${categoryDomains.length} category pages`,
);
