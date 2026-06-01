import { PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/math-signal-core-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "math-signal-core",
  title: "Math Signal Core",
  description: "Shared signal-domain math for windows, frame strides, resampling, and biquad design.",
  domain: "math",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/math-signal-core",
    standaloneRoute: "",
  },
  defaultOperation: "signal.frames",
  featuredOperations: ["signal.frames", "signal.filterDesign", "signal.resamplePlan", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["signal.frames", "signal.filterDesign"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe", "signal.resamplePlan"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
