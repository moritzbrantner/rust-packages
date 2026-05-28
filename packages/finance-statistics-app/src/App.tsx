import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/finance-statistics-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "finance-statistics",
  title: "Finance Statistics",
  description: "Finance-oriented return, risk, and rolling statistics helpers.",
  domain: "math",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/finance-statistics",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
