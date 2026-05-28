import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/three-d-scene-svg-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "three-d-scene-svg",
  title: "Three D Scene Svg",
  description: "SVG-inspired declarative 3D scene documents and deterministic SVG preview rendering.",
  domain: "three-d",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/three-d-scene-svg",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
