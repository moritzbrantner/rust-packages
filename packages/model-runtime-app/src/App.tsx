import { PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/model-runtime-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "model-runtime",
  title: "Model Runtime",
  description: "Generic model specs, bundles, downloads, and job helpers for multimodal runtimes.",
  domain: "runtime",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/model-runtime",
    standaloneRoute: "",
  },
  defaultOperation: "model.presets",
  featuredOperations: ["model.presets", "model.spec", "model.bundlePlan", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["model.presets", "model.spec"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe", "model.bundlePlan"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
