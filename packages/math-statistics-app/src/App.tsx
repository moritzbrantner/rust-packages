import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/math-statistics-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "math-statistics",
  title: "Math Statistics",
  description: "Shared multivariate statistics for dense matrix inputs and streaming observations.",
  domain: "math",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/math-statistics",
    standaloneRoute: "",
  },
  defaultOperation: "stats.normalize",
  featuredOperations: ["stats.normalize", "stats.covariance", "stats.pca", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["stats.normalize", "stats.covariance", "stats.pca"],
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
