import { PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/vector-analysis-index-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "vector-analysis-index",
  title: "Vector Analysis Index",
  description: "Exact in-memory vector search for video-analysis.",
  domain: "vector",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/vector-analysis-index",
    standaloneRoute: "",
  },
  defaultOperation: "vector.index.search",
  featuredOperations: ["vector.index.search", "vector.index.centroids", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["vector.index.search", "vector.index.centroids"],
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
