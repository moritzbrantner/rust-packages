import { PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/video-analysis-core-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "video-analysis-core",
  title: "Video Analysis Core",
  description: "Core media, timing, detection, and analyzer contracts for video-analysis.",
  domain: "video",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/video-analysis-core",
    standaloneRoute: "",
  },
  defaultOperation: "video.core.frameSummary",
  featuredOperations: ["video.core.frameSummary", "video.core.timecode", "video.core.sceneSummary", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["video.core.frameSummary", "video.core.timecode"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe", "video.core.sceneSummary"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
