import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/geo-viz-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "geo-viz",
  title: "Geo Viz Core",
  description: "Renderer-agnostic geographic visualization indexes for maps.",
  domain: "video",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/geo-viz",
    standaloneRoute: "",
  },
  defaultOperation: "describe",
  featuredOperations: ["describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: [],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
