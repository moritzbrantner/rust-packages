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
  defaultOperation: "jobs.lifecycle",
  featuredOperations: ["jobs.lifecycle", "jobs.manifest", "jobs.spec", "jobs.progress", "jobs.events", "jobs.artifactValidate", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["jobs.lifecycle", "jobs.manifest"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe", "jobs.spec", "jobs.progress", "jobs.events", "jobs.artifactValidate"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
