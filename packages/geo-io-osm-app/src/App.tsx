import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/geo-io-osm-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "geo-io-osm",
  title: "OSM Data",
  description: "OpenStreetMap PBF import adapters for geo-core.",
  domain: "data",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/geo-io-osm",
    standaloneRoute: "",
  },
  defaultOperation: "osm.filterPbfBase64",
  featuredOperations: ["osm.filterPbfBase64", "osm.filterSummary", "osm.validateSpec", "describe"],
  fileInputs: [
    {
      id: "osm-pbf",
      label: "OSM PBF",
      accept: ".osm.pbf,.pbf,application/octet-stream",
      targetPath: ["pbfBase64"],
    },
  ],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["osm.filterPbfBase64"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["osm.filterSummary", "osm.validateSpec", "describe"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
