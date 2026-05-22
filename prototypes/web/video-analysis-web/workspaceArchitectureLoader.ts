import { spawn } from "node:child_process";
import { readdir, readFile } from "node:fs/promises";
import { dirname, relative } from "node:path";

import {
  packageDomainFor,
  packageDomainOrder,
  type WorkspaceArchitectureDependency,
  type WorkspaceArchitecturePackage,
  type WorkspaceArchitectureResponse,
} from "./src/workspaceArchitecture";
import {
  buildInteropPairs,
  extractContractTags,
  parseWorkspaceContractMap,
  splitConsumedBy,
  splitExposes,
  type ContractMapRow,
} from "./src/workspaceArchitectureServer";

interface CargoMetadataPackage {
  id: string;
  name: string;
  description: string | null;
  manifest_path: string;
  dependencies: CargoMetadataDependency[];
  targets: CargoMetadataTarget[];
}

interface CargoMetadataDependency {
  name: string;
  kind: string | null;
  optional: boolean;
  path?: string | null;
}

interface CargoMetadataTarget {
  kind: string[];
  name: string;
}

interface CargoMetadataResponse {
  packages: CargoMetadataPackage[];
  workspace_members: string[];
}

interface FrontendPackageJson {
  name?: string;
  description?: string;
}

const architectureDocPath = "docs/API_CONTRACTS.md";

export async function loadWorkspaceArchitecture(workspaceRoot: string): Promise<WorkspaceArchitectureResponse> {
  const [metadata, contractMarkdown, uiPackageJson, webPackageJson] = await Promise.all([
    cargoMetadata(workspaceRoot),
    readFile(`${workspaceRoot}/${architectureDocPath}`, "utf8"),
    readFile(`${workspaceRoot}/packages/video-analysis-ui/package.json`, "utf8"),
    readFile(`${workspaceRoot}/prototypes/web/video-analysis-web/package.json`, "utf8"),
  ]);

  const contractRows = parseWorkspaceContractMap(contractMarkdown);
  const contractByName = new Map(contractRows.map((row) => [row.name, row]));
  const workspaceMemberIds = new Set(metadata.workspace_members);
  const cargoPackages: CargoMetadataPackage[] = metadata.packages.filter((pkg: CargoMetadataPackage) =>
    workspaceMemberIds.has(pkg.id),
  );
  const includedNames = new Set<string>([
    ...cargoPackages.map((pkg: CargoMetadataPackage) => pkg.name),
    ...contractRows.map((row) => row.name),
    "@video-analysis/web",
    "@video-analysis/ui",
  ]);

  const packages: WorkspaceArchitecturePackage[] = cargoPackages.map((pkg: CargoMetadataPackage) =>
    toArchitecturePackage(workspaceRoot, pkg, contractByName.get(pkg.name)),
  );

  const uiInfo = JSON.parse(uiPackageJson) as { name?: string };
  const webInfo = JSON.parse(webPackageJson) as { name?: string };

  if (!packages.some((pkg) => pkg.name === "@video-analysis/ui")) {
    packages.push({
      name: uiInfo.name ?? "@video-analysis/ui",
      kind: "frontend",
      domain: "ui",
      path: "packages/video-analysis-ui",
      description: "React/Tailwind component pack for rendered analysis reports.",
      role:
        contractByName.get("@video-analysis/ui")?.role ??
        "React and Tailwind views for analysis data and report JSON.",
      exposes: splitExposes(contractByName.get("@video-analysis/ui")?.exposesText),
      consumedBy: splitConsumedBy(contractByName.get("@video-analysis/ui")?.consumedByText),
      tags: extractContractTags(
        [
          contractByName.get("@video-analysis/ui")?.role,
          contractByName.get("@video-analysis/ui")?.exposesText,
          "report views scenes observations transcript data buckets dashboard json report",
        ]
          .filter(Boolean)
          .join(" "),
      ),
      capabilities: capabilitiesFor("@video-analysis/ui", "frontend", "packages/video-analysis-ui", true),
    });
  }

  if (!packages.some((pkg) => pkg.name === "@video-analysis/web")) {
    packages.push({
      name: webInfo.name ?? "@video-analysis/web",
      kind: "frontend",
      domain: "apps",
      path: "prototypes/web/video-analysis-web",
      description: "Interactive prototype testbed for use cases, flows, and package architecture.",
      role: "Prototype project that drives the UI package and local Rust workflows.",
      exposes: ["Interactive run, flow, result, and architecture views"],
      consumedBy: ["Developers exploring workspace behavior locally"],
      tags: extractContractTags("dashboard report workflow json report scenes observations data buckets"),
      capabilities: capabilitiesFor(
        "@video-analysis/web",
        "frontend",
        "prototypes/web/video-analysis-web",
        true,
      ),
    });
  }

  for (const frontendPackage of await loadFrontendPackages(workspaceRoot)) {
    if (packages.some((pkg) => pkg.name === frontendPackage.name)) {
      continue;
    }
    const contract = contractByName.get(frontendPackage.name);
    const description = frontendPackage.description || frontendRole(frontendPackage.name);
    const exposes = splitExposes(contract?.exposesText);
    packages.push({
      name: frontendPackage.name,
      kind: "frontend",
      domain: packageDomainFor(frontendPackage.name, frontendPackage.path),
      path: frontendPackage.path,
      description,
      role: contract?.role ?? description,
      exposes: exposes.length > 0 ? exposes : frontendExposes(frontendPackage.name),
      consumedBy: splitConsumedBy(contract?.consumedByText),
      tags: extractContractTags([contract?.role, contract?.exposesText, description, frontendPackage.name].filter(Boolean).join(" ")),
      capabilities: capabilitiesFor(frontendPackage.name, "frontend", frontendPackage.path, true),
    });
  }

  const dependencySet = new Set<string>();
  const dependencies: WorkspaceArchitectureDependency[] = [];

  for (const pkg of cargoPackages) {
    for (const dependency of pkg.dependencies) {
      if (dependency.kind === "dev" || !dependency.path || !includedNames.has(dependency.name)) {
        continue;
      }
      const key = `${pkg.name}->${dependency.name}`;
      if (dependencySet.has(key)) {
        continue;
      }
      dependencySet.add(key);
      dependencies.push({
        source: pkg.name,
        target: dependency.name,
        optional: dependency.optional,
      });
    }
  }

  dependencies.push({
    source: "@video-analysis/web",
    target: "@video-analysis/ui",
    optional: false,
  });

  const packageNames = new Set(packages.map((pkg) => pkg.name));
  const filteredDependencies = dependencies.filter(
    (dependency) => packageNames.has(dependency.source) && packageNames.has(dependency.target),
  );
  const interop = buildInteropPairs(packages, filteredDependencies);

  packages.sort(sortPackages);
  filteredDependencies.sort((left, right) =>
    left.source === right.source ? left.target.localeCompare(right.target) : left.source.localeCompare(right.source),
  );
  interop.sort((left, right) =>
    right.strength === left.strength
      ? `${left.packages[0]}:${left.packages[1]}`.localeCompare(`${right.packages[0]}:${right.packages[1]}`)
      : right.strength - left.strength,
  );

  return {
    generatedAt: new Date().toISOString(),
    packages,
    dependencies: filteredDependencies,
    interop,
  };
}

function toArchitecturePackage(
  workspaceRoot: string,
  pkg: CargoMetadataPackage,
  contract: ContractMapRow | undefined,
): WorkspaceArchitecturePackage {
  const path = relativePath(workspaceRoot, pkg.manifest_path);
  const description = pkg.description ?? contract?.role ?? "";
  const exposes = splitExposes(contract?.exposesText);
  const consumedBy = splitConsumedBy(contract?.consumedByText);
  const hasLibraryTarget = pkg.targets.some((target) => target.kind.includes("lib"));

  return {
    name: pkg.name,
    kind: "rust",
    domain: packageDomainFor(pkg.name, path),
    path,
    description,
    role: contract?.role ?? description,
    exposes,
    consumedBy,
    tags: extractContractTags([contract?.role, contract?.exposesText, description].filter(Boolean).join(" ")),
    capabilities: capabilitiesFor(pkg.name, "rust", path, hasLibraryTarget),
  };
}

function capabilitiesFor(
  name: string,
  kind: "rust" | "frontend",
  path: string | null,
  hasLibraryTarget: boolean,
): WorkspaceArchitecturePackage["capabilities"] {
  if (kind === "frontend") {
    const capabilities: WorkspaceArchitecturePackage["capabilities"] = [
      {
        kind: "library",
        entrypoint: libraryEntrypoint(name, kind, path, hasLibraryTarget),
      },
    ];
    if (name.endsWith("-wasm")) {
      capabilities.push({
        kind: "wasm",
        entrypoint: `import from ${name}`,
      });
    }
    if (name.endsWith("-app") || name === "@video-analysis/web" || name === "@video-analysis/ui") {
      capabilities.push({
        kind: "ui",
        entrypoint: path ?? name,
      });
    }
    return capabilities;
  }
  if (name.endsWith("-wasm") || path?.includes("/bindings/")) {
    return [
      {
        kind: "library",
        entrypoint: libraryEntrypoint(name, kind, path, hasLibraryTarget),
      },
      {
        kind: "wasm",
        entrypoint: `wasm-pack build ${path}`,
      },
    ];
  }

  const capabilities: WorkspaceArchitecturePackage["capabilities"] = [
    {
      kind: "library",
      entrypoint: libraryEntrypoint(name, kind, path, hasLibraryTarget),
    },
    {
      kind: "cli",
      entrypoint:
        kind === "rust"
          ? `${name}/cli (package ${name}-cli)`
          : "frontend package scripts",
    },
    {
      kind: "api",
      entrypoint: `${name}/api (package ${name}-server)`,
    },
    {
      kind: "ui",
      entrypoint: `${name}/app (package ${name}-app)`,
    },
  ];

  return capabilities;
}

function libraryEntrypoint(
  name: string,
  kind: "rust" | "frontend",
  path: string | null,
  hasLibraryTarget: boolean,
): string {
  if (kind === "frontend") {
    return name === "@video-analysis/ui" ? "import from @video-analysis/ui" : path ?? name;
  }
  if (hasLibraryTarget) {
    return `use ${name.replaceAll("-", "_")}`;
  }
  return "add src/lib.rs before publishing new library APIs";
}

async function loadFrontendPackages(
  workspaceRoot: string,
): Promise<Array<{ name: string; path: string; description: string }>> {
  const packageRoot = `${workspaceRoot}/packages`;
  const entries = await readdir(packageRoot, { withFileTypes: true });
  const packages = await Promise.all(
    entries
      .filter((entry) => entry.isDirectory())
      .map(async (entry) => {
        const path = `packages/${entry.name}`;
        try {
          const parsed = JSON.parse(
            await readFile(`${packageRoot}/${entry.name}/package.json`, "utf8"),
          ) as FrontendPackageJson;
          return {
            name: parsed.name ?? entry.name,
            path,
            description: parsed.description ?? frontendRole(parsed.name ?? entry.name),
          };
        } catch {
          return null;
        }
      }),
  );
  return packages.filter((pkg): pkg is { name: string; path: string; description: string } => Boolean(pkg));
}

function frontendRole(name: string): string {
  if (name.endsWith("-wasm")) {
    return `WASM bindings for ${frontendLibraryName(name)}.`;
  }
  if (name.endsWith("-app")) {
    return `React package app for ${frontendLibraryName(name)}.`;
  }
  return `Frontend package for ${name}.`;
}

function frontendExposes(name: string): string[] {
  if (name.endsWith("-wasm")) {
    return ["Browser import surface for Rust package functions"];
  }
  if (name.endsWith("-app")) {
    return ["Interactive React package surface"];
  }
  return ["Frontend package exports"];
}

function frontendLibraryName(name: string): string {
  return name.replace(/^@mb-rust\//, "").replace(/-(app|wasm)$/, "");
}

function cargoMetadata(workspaceRoot: string): Promise<CargoMetadataResponse> {
  return new Promise((resolve, reject) => {
    const child = spawn("cargo", ["metadata", "--format-version", "1", "--no-deps"], {
      cwd: workspaceRoot,
      env: process.env,
      stdio: ["ignore", "pipe", "pipe"],
    });

    let stdout = "";
    let stderr = "";

    child.stdout.on("data", (chunk) => {
      stdout += chunk.toString("utf8");
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString("utf8");
    });
    child.on("error", reject);
    child.on("close", (exitCode) => {
      if (exitCode !== 0) {
        reject(new Error(stderr.trim() || stdout.trim() || "cargo metadata failed"));
        return;
      }
      try {
        resolve(JSON.parse(stdout) as CargoMetadataResponse);
      } catch (error) {
        reject(error);
      }
    });
  });
}

function relativePath(workspaceRoot: string, manifestPath: string): string {
  const packageDir = dirname(manifestPath);
  return relative(workspaceRoot, packageDir).replaceAll("\\", "/");
}

function sortPackages(left: WorkspaceArchitecturePackage, right: WorkspaceArchitecturePackage): number {
  const domainDistance =
    packageDomainOrder.indexOf(left.domain) - packageDomainOrder.indexOf(right.domain);
  if (domainDistance !== 0) {
    return domainDistance;
  }
  return left.name.localeCompare(right.name);
}
