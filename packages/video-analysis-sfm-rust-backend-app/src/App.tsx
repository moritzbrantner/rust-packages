import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/video-analysis-sfm-rust-backend-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "video-analysis-sfm-rust-backend",
  title: "Video Analysis SFM Rust Backend",
  description: "Rust-native SfM backend adapters for video-analysis.",
  domain: "video",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/video-analysis-sfm-rust-backend",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
