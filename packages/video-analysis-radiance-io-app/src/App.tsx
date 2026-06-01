import { PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/video-analysis-radiance-io-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "video-analysis-radiance-io",
  title: "Video Analysis Radiance IO",
  description: "COLMAP, Nerfstudio, and PLY I/O for video-analysis radiance workflows.",
  domain: "video",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/video-analysis-radiance-io",
    standaloneRoute: "",
  },
  defaultOperation: "radiance.io.colmapCameraSupport",
  featuredOperations: [
    "radiance.io.colmapCameraSupport",
    "radiance.io.colmapSummary",
    "radiance.io.gaussianSplatSummary",
    "describe",
  ],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["radiance.io.colmapCameraSupport"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe", "radiance.io.colmapSummary", "radiance.io.gaussianSplatSummary"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
