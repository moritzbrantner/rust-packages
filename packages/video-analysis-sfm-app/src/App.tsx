import { PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/video-analysis-sfm-wasm";

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
  defaultOperation: "video.sfm.reconstruct",
  featuredOperations: ["video.sfm.reconstruct", "video.sfm.matchPlan", "video.sfm.cameraGraph", "video.sfm.reconstructionSummary", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["video.sfm.reconstruct"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: [
        "describe",
        "video.sfm.matchPlan",
        "video.sfm.cameraGraph",
        "video.sfm.reconstructionSummary",
        "video.colmap.commandPlan",
        "video.colmap.imageList",
        "video.colmap.sparseSummary",
        "video.colmap.reconstructVideo",
        "video.sfmRust.matchSummary",
        "video.sfmRust.trackPlan",
        "video.sfmRust.bundlePlan",
        "video.opencv.sfmPlan",
      ],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
