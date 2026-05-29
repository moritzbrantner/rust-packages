import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/video-analysis-mvs-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "video-analysis-mvs",
  title: "Video Analysis MVS",
  description: "Multi-View Stereo backend contracts and dense reconstruction outputs for video-analysis.",
  domain: "video",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/video-analysis-mvs",
    standaloneRoute: "",
  },
  defaultOperation: "video.mvs.depthPlan",
  featuredOperations: ["video.mvs.depthPlan", "video.mvs.fusionPlan", "video.mvs.outputSummary", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["video.mvs.depthPlan"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe", "video.mvs.fusionPlan", "video.mvs.outputSummary"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
