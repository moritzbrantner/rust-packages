import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/vector-analysis-core-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "vector-analysis-core",
  title: "Vector Analysis Core",
  description: "Dense vector validation and metrics for video-analysis.",
  domain: "vector",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/vector-analysis-core",
    standaloneRoute: "",
  },
  defaultOperation: "vector.normalize",
  featuredOperations: ["vector.normalize", "vector.distance", "vector.summary", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["vector.normalize", "vector.distance"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe", "vector.summary"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
