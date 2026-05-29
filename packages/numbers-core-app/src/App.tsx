import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/numbers-core-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "numbers-core",
  title: "Numbers Core",
  description: "Deterministic scalar numeric summaries, quantiles, ranges, and histograms for video-analysis.",
  domain: "data",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/numbers-core",
    standaloneRoute: "",
  },
  defaultOperation: "numbers.summary",
  featuredOperations: ["numbers.summary", "numbers.quantiles", "numbers.histogram", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["numbers.summary", "numbers.quantiles"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe", "numbers.histogram"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
