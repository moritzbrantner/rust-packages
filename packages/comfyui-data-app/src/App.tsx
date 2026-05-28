import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/comfyui-data-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "comfyui-data",
  title: "Comfyui Data",
  description: "Serde contracts for ComfyUI workflow and prompt data.",
  domain: "comfyui",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/comfyui-data",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
