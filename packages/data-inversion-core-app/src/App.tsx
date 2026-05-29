import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/data-inversion-core-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "data-inversion-core",
  title: "Data Inversion Core",
  description: "Shared fidelity and inversion trace metadata for generated analysis outputs.",
  domain: "data",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/data-inversion-core",
    standaloneRoute: "",
  },
  defaultOperation: "inversion.trace",
  featuredOperations: ["inversion.trace", "inversion.confidence", "inversion.fidelity", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["inversion.trace", "inversion.confidence", "inversion.fidelity"],
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
