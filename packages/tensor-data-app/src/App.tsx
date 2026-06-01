import { PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/tensor-data-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "tensor-data",
  title: "Tensor Data",
  description: "Small finite f32 tensor contracts and metadata for video-analysis.",
  domain: "data",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/tensor-data",
    standaloneRoute: "",
  },
  defaultOperation: "tensor.validate",
  featuredOperations: ["tensor.validate", "tensor.summary", "tensor.reshapePlan", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["tensor.validate"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe", "tensor.summary", "tensor.reshapePlan"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
