import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/jobs-core-wasm";

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
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
