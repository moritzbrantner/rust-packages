import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/geo-io-geojson-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "geo-io-geojson",
  title: "Geo Data",
  description: "GeoJSON import and export adapters for geo-core.",
  domain: "data",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/geo-io-geojson",
    standaloneRoute: "",
  },
  defaultOperation: "geo.bounds",
  featuredOperations: ["geo.bounds", "geo.distance", "geo.toGeoJson", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["geo.bounds", "geo.distance", "geo.toGeoJson"],
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
