import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/video-analysis-sfm-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "video-analysis-sfm",
  title: "Video Analysis SFM",
  description: "Structure-from-Motion backend contracts and sparse pipeline orchestration for video-analysis.",
  domain: "video",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/video-analysis-sfm",
    standaloneRoute: "",
  },
  defaultOperation: "video.sfm.matchPlan",
  featuredOperations: ["video.sfm.matchPlan", "video.sfm.cameraGraph", "video.sfm.reconstructionSummary", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["video.sfm.matchPlan"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe", "video.sfm.cameraGraph", "video.sfm.reconstructionSummary"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
