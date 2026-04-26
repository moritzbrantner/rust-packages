import {
  contractTagDefinitions,
  type WorkspaceArchitectureDependency,
  type WorkspaceArchitectureInterop,
  type WorkspaceArchitecturePackage,
} from "./workspaceArchitecture";

export interface ContractMapRow {
  name: string;
  role: string;
  dependsOnText: string;
  exposesText: string;
  consumedByText: string;
}

export function parseWorkspaceContractMap(markdown: string): ContractMapRow[] {
  const heading = "## Workspace Contract Map";
  const headingIndex = markdown.indexOf(heading);
  if (headingIndex === -1) {
    return [];
  }
  const section = markdown.slice(headingIndex + heading.length).split("\n");
  const tableLines: string[] = [];
  let started = false;
  for (const line of section) {
    const trimmed = line.trim();
    if (!started) {
      if (trimmed.startsWith("| Package |")) {
        started = true;
        tableLines.push(line);
      }
      continue;
    }
    if (!trimmed.startsWith("|")) {
      break;
    }
    tableLines.push(line);
  }

  return tableLines
    .slice(2)
    .map(splitMarkdownTableRow)
    .filter((row): row is string[] => row.length >= 5)
    .map((row) => ({
      name: cleanMarkdownCell(row[0]),
      role: cleanMarkdownCell(row[1]),
      dependsOnText: cleanMarkdownCell(row[2]),
      exposesText: cleanMarkdownCell(row[3]),
      consumedByText: cleanMarkdownCell(row[4]),
    }));
}

export function extractContractTags(text: string): string[] {
  const normalized = normalizeText(text);
  return contractTagDefinitions
    .filter((definition) =>
      definition.terms.some((term) => normalized.includes(normalizeText(term))),
    )
    .map((definition) => definition.id);
}

export function buildInteropPairs(
  packages: WorkspaceArchitecturePackage[],
  dependencies: WorkspaceArchitectureDependency[],
): WorkspaceArchitectureInterop[] {
  const dependencyKeys = new Set(dependencies.map((dependency) => `${dependency.source}->${dependency.target}`));
  const interop: WorkspaceArchitectureInterop[] = [];

  for (let index = 0; index < packages.length; index += 1) {
    for (let cursor = index + 1; cursor < packages.length; cursor += 1) {
      const left = packages[index];
      const right = packages[cursor];
      const forward = dependencyKeys.has(`${left.name}->${right.name}`);
      const reverse = dependencyKeys.has(`${right.name}->${left.name}`);
      const sharedTags = intersect(left.tags, right.tags);
      if (!forward && !reverse && sharedTags.length === 0) {
        continue;
      }

      const reasons = [
        ...(forward ? [`${left.name} depends on ${right.name}`] : []),
        ...(reverse ? [`${right.name} depends on ${left.name}`] : []),
        ...sharedTags.map(
          (tagId) => contractTagDefinitions.find((definition) => definition.id === tagId)?.label ?? tagId,
        ),
      ];

      interop.push({
        packages: [left.name, right.name],
        directDependency: forward || reverse,
        sharedTags,
        reasons,
        strength: sharedTags.length + (forward || reverse ? 2 : 0),
      });
    }
  }

  return interop;
}

export function splitExposes(value: string | undefined): string[] {
  if (!value) {
    return [];
  }
  return value
    .split(",")
    .map((entry) => entry.trim())
    .filter(Boolean);
}

export function splitConsumedBy(value: string | undefined): string[] {
  if (!value) {
    return [];
  }
  return value
    .split(/,| and /)
    .map((entry) => entry.trim())
    .filter(Boolean);
}

function splitMarkdownTableRow(line: string): string[] {
  return line
    .trim()
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split("|")
    .map((cell) => cell.trim());
}

function cleanMarkdownCell(value: string): string {
  return value
    .replace(/\[([^\]]+)\]\([^)]+\)/g, "$1")
    .replace(/`/g, "")
    .replace(/\s+/g, " ")
    .trim();
}

function normalizeText(value: string): string {
  return value.toLowerCase().replace(/[^a-z0-9]+/g, " ").trim();
}

function intersect(left: string[], right: string[]): string[] {
  const rightSet = new Set(right);
  return left.filter((entry, index) => rightSet.has(entry) && left.indexOf(entry) === index);
}
