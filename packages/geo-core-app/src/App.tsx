import { PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/geo-core-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "geo-core",
  title: "Geo Data",
  description: "GeoJSON-oriented geometry data structures, processing algorithms, and transforms for video-analysis.",
  domain: "data",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/geo-core",
    standaloneRoute: "",
  },
  defaultOperation: "geo.distance",
  featuredOperations: ["geo.distance", "geo.bounds", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["geo.distance", "geo.bounds"],
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
