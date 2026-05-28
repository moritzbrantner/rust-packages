import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/runtime-artifacts-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "runtime-artifacts",
  title: "Runtime Artifacts",
  description: "Shared artifact DTOs and minimal local artifact stores for runtime operations.",
  domain: "runtime",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/runtime-artifacts",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
