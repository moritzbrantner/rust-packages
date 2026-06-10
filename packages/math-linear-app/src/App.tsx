import { PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/math-linear-wasm";

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
  featuredOperations: [
    "linear.matmul",
    "linear.transpose",
    "linear.solve",
    "linear.decompose",
    "linear.inverse",
    "linear.kernel1d",
    "linear.tensorBridge",
    "linear.gram",
    "linear.cholesky",
    "linear.qr",
    "linear.center",
    "linear.leastSquares",
    "linear.svd",
    "linear.pseudoinverse",
    "linear.rank",
    "describe",
  ],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: [
        "linear.matmul",
        "linear.transpose",
        "linear.solve",
        "linear.decompose",
        "linear.inverse",
        "linear.kernel1d",
        "linear.tensorBridge",
        "linear.gram",
        "linear.cholesky",
        "linear.qr",
        "linear.center",
        "linear.leastSquares",
        "linear.svd",
        "linear.pseudoinverse",
        "linear.rank",
      ],
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
