import { PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/video-analysis-radiance-fields-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "video-analysis-radiance-fields",
  title: "Video Analysis Radiance Fields",
  description: "Radiance-field cameras, rays, and volumes for video-analysis.",
  domain: "video",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/video-analysis-radiance-fields",
    standaloneRoute: "",
  },
  defaultOperation: "video.radiance.fieldSummary",
  featuredOperations: ["video.radiance.fieldSummary", "video.radiance.cameraPath", "video.radiance.renderPlan", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["video.radiance.fieldSummary", "video.radiance.cameraPath"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe", "video.radiance.renderPlan"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
