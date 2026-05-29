import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/math-linear-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "math-linear",
  title: "Math Linear",
  description: "Dense matrix and kernel contracts bridging tensor-data and vector-analysis-core.",
  domain: "math",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/math-linear",
    standaloneRoute: "",
  },
  defaultOperation: "linear.matmul",
  featuredOperations: ["linear.matmul", "linear.kernel1d", "linear.tensorBridge", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["linear.matmul", "linear.kernel1d", "linear.tensorBridge"],
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
