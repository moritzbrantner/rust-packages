import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/math-signal-core-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "math-signal-core",
  title: "Math Signal Core",
  description: "Shared signal-domain math for windows, frame strides, resampling, and biquad design.",
  domain: "math",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/math-signal-core",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
