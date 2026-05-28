import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/three-d-processing-io-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "three-d-processing-io",
  title: "Three D Processing IO",
  description: "Mesh and point-cloud interchange formats for video-analysis.",
  domain: "three-d",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/three-d-processing-io",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
