import { PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/dense-data-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "dense-data",
  title: "Dense Data",
  description: "Deterministic dense point datasets, bucketing, and clustering for video-analysis.",
  domain: "data",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/dense-data",
    standaloneRoute: "",
  },
  defaultOperation: "summarizeDensePoints",
  featuredOperations: ["summarizeDensePoints", "bucketDensePoints", "clusterDensePoints", "binNumericSeries", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["summarizeDensePoints", "bucketDensePoints", "clusterDensePoints", "binNumericSeries"],
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
