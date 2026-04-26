import { describe, expect, it } from "vitest";

import { buildInteropPairs, extractContractTags, parseWorkspaceContractMap } from "./workspaceArchitectureServer";
import type { WorkspaceArchitectureDependency, WorkspaceArchitecturePackage } from "./workspaceArchitecture";

describe("workspace architecture server helpers", () => {
  it("parses the contract table rows from markdown", () => {
    const rows = parseWorkspaceContractMap(`
## Workspace Contract Map

| Package | Role | Depends on | Exposes | Consumed by |
| --- | --- | --- | --- | --- |
| \`video-analysis-core\` | Canonical shared contracts | External utility crates only | Time/frame types, observations, metrics | All functional Rust crates |
| \`@video-analysis/ui\` | React views | React peer deps | Report types and component exports | Web apps |

## Canonical Core Contracts
`);

    expect(rows).toHaveLength(2);
    expect(rows[0]?.name).toBe("video-analysis-core");
    expect(rows[1]?.name).toBe("@video-analysis/ui");
  });

  it("extracts contract tags from exposed surfaces", () => {
    expect(
      extractContractTags("Time/frame types, observations, transcript segments, JSON report writers"),
    ).toEqual(expect.arrayContaining(["observations", "text_segments", "reports_and_outputs"]));
  });

  it("builds interop pairs from dependency edges and shared tags", () => {
    const packages: WorkspaceArchitecturePackage[] = [
      {
        name: "video-analysis-core",
        kind: "rust",
        domain: "video",
        path: "crates/video/video-analysis-core",
        description: "",
        role: "",
        exposes: [],
        consumedBy: [],
        tags: ["video_frames", "observations"],
      },
      {
        name: "video-analysis-detectors",
        kind: "rust",
        domain: "video",
        path: "crates/video/video-analysis-detectors",
        description: "",
        role: "",
        exposes: [],
        consumedBy: [],
        tags: ["scenes", "video_frames"],
      },
      {
        name: "text-analysis-core",
        kind: "rust",
        domain: "text",
        path: "crates/text/text-analysis-core",
        description: "",
        role: "",
        exposes: [],
        consumedBy: [],
        tags: ["text_segments"],
      },
    ];
    const dependencies: WorkspaceArchitectureDependency[] = [
      { source: "video-analysis-detectors", target: "video-analysis-core", optional: false },
    ];

    const relations = buildInteropPairs(packages, dependencies);
    const detectorPair = relations.find((relation) =>
      relation.packages.includes("video-analysis-core") && relation.packages.includes("video-analysis-detectors"),
    );

    expect(detectorPair).toBeDefined();
    expect(detectorPair?.directDependency).toBe(true);
    expect(detectorPair?.sharedTags).toContain("video_frames");
    expect(relations.some((relation) => relation.packages.includes("text-analysis-core"))).toBe(false);
  });
});
