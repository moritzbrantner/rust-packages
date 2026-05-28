import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/maps-kernels-core-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "maps-kernels-core",
  title: "Maps Kernels Core",
  description: "Numeric kernels for map and temporal GeoJSON processing.",
  domain: "math",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/maps-kernels-core",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
