import { PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/graph-analysis-core-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "graph-analysis-core",
  title: "Graph Analysis Core",
  description: "Graph and tree analysis primitives for video-analysis.",
  domain: "data",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/graph-analysis-core",
    standaloneRoute: "",
  },
  defaultOperation: "graph.components",
  featuredOperations: ["graph.components", "graph.shortestPath", "graph.validateTree", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["graph.components", "graph.shortestPath"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe", "graph.validateTree"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
