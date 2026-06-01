import { PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/math-sparse-data-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "math-sparse-data",
  title: "Math Sparse Data",
  description: "Sparse vector and matrix contracts for text, retrieval, and feature indexing.",
  domain: "math",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/math-sparse-data",
    standaloneRoute: "",
  },
  defaultOperation: "sparse.similarity",
  featuredOperations: ["sparse.similarity", "sparse.toDense", "sparse.matrixSummary", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["sparse.similarity", "sparse.toDense"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe", "sparse.matrixSummary"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
