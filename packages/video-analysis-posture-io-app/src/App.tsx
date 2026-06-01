import { PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/video-analysis-posture-io-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "video-analysis-posture-io",
  title: "Video Analysis Posture IO",
  description: "COCO-style posture I/O and 3D stick-figure export for video-analysis.",
  domain: "video",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/video-analysis-posture-io",
    standaloneRoute: "",
  },
  defaultOperation: "video.postureIo.formatSummary",
  featuredOperations: ["video.postureIo.formatSummary", "video.postureIo.parsePlan", "video.postureIo.exportPlan", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["video.postureIo.formatSummary"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe", "video.postureIo.parsePlan", "video.postureIo.exportPlan"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
