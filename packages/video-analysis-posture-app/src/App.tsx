import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/video-analysis-posture-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "video-analysis-posture",
  title: "Video Analysis Posture",
  description: "Pose and skeleton helpers for video-analysis.",
  domain: "video",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/video-analysis-posture",
    standaloneRoute: "",
  },
  defaultOperation: "video.posture.keypointSummary",
  featuredOperations: ["video.posture.keypointSummary", "video.posture.skeletonPlan", "video.posture.motionSummary", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["video.posture.keypointSummary"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe", "video.posture.skeletonPlan", "video.posture.motionSummary"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
