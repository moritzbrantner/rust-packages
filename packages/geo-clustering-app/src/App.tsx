import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/geo-clustering-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "geo-clustering",
  title: "Geo Data",
  description: "GeoJSON-oriented geometry data structures, processing algorithms, and transforms for video-analysis.",
  domain: "data",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/geo-clustering",
    standaloneRoute: "",
  },
  defaultOperation: "geoCluster.viewport",
  featuredOperations: ["geoCluster.viewport", "geoCluster.bounds", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["geoCluster.viewport", "geoCluster.bounds"],
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
