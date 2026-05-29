import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/video-analysis-output-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "video-analysis-output",
  title: "Video Analysis Output",
  description: "CSV and HTML report helpers for video-analysis.",
  domain: "video",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/video-analysis-output",
    standaloneRoute: "",
  },
  defaultOperation: "video.output.reportSummary",
  featuredOperations: ["video.output.reportSummary", "video.output.csvPlan", "video.output.htmlPlan", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["video.output.reportSummary"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe", "video.output.csvPlan", "video.output.htmlPlan"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
