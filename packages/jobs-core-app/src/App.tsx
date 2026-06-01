import { PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/jobs-core-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "jobs-core",
  title: "Jobs Core",
  description: "Reusable long-running job state, cancellation, progress, logs, and artifact primitives.",
  domain: "jobs",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/jobs-core",
    standaloneRoute: "",
  },
  defaultOperation: "jobs.spec",
  featuredOperations: ["jobs.spec", "jobs.progress", "jobs.lifecycle", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["jobs.spec", "jobs.progress", "jobs.lifecycle"],
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
