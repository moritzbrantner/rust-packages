import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/runtime-jobs-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "runtime-jobs",
  title: "Runtime Jobs",
  description: "Shared serializable job and operation result DTOs for runtime surfaces.",
  domain: "runtime",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/runtime-jobs",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
