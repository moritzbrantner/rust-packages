import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/video-analysis-test-support-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "video-analysis-test-support",
  title: "Video Analysis Test Support",
  description: "Runtime surface for the video-analysis-test-support library crate.",
  domain: "support",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/video-analysis-test-support",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
