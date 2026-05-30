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
  defaultOperation: "geoViz.aggregateViewport",
  featuredOperations: [
    "geoViz.aggregateViewport",
    "geoViz.bounds",
    "geoViz.heatViewport",
    "geoViz.geoJsonViewport",
    "geoViz.flowViewport",
    "geoViz.resampleGeometry",
    "describe",
  ],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: [
        "geoViz.aggregateViewport",
        "geoViz.bounds",
        "geoViz.heatViewport",
        "geoViz.geoJsonViewport",
        "geoViz.flowViewport",
        "geoViz.resampleGeometry",
      ],
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
