import { PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/video-analysis-reconstruction-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "video-analysis-reconstruction",
  title: "Video Analysis Reconstruction",
  description: "Sparse reconstruction helpers for video-analysis.",
  domain: "video",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/video-analysis-reconstruction",
    standaloneRoute: "",
  },
  defaultOperation: "video.reconstruction.plan",
  featuredOperations: [
    "video.reconstruction.plan",
    "video.reconstruction.cameraSummary",
    "video.reconstruction.assetSummary",
    "describe",
  ],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["video.reconstruction.plan"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe", "video.reconstruction.cameraSummary", "video.reconstruction.assetSummary"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
