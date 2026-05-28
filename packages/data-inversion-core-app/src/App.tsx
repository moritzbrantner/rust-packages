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
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
