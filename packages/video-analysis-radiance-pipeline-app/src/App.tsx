import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/video-analysis-radiance-pipeline-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "video-analysis-radiance-pipeline",
  title: "Video Analysis Radiance Pipeline",
  description: "Typed radiance project loading, validation, summaries, and CPU previews for video-analysis.",
  domain: "video",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/video-analysis-radiance-pipeline",
    standaloneRoute: "",
  },
  defaultOperation: "video.radiancePipeline.stagePlan",
  featuredOperations: [
    "video.radiancePipeline.stagePlan",
    "video.radiancePipeline.assetCheck",
    "video.radiancePipeline.commandPlan",
    "describe",
  ],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["video.radiancePipeline.stagePlan"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe", "video.radiancePipeline.assetCheck", "video.radiancePipeline.commandPlan"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
