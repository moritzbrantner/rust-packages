import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/comfyui-models-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "comfyui-models",
  title: "Comfyui Models",
  description: "ComfyUI model folder, inventory, and extra path contracts.",
  domain: "comfyui",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/comfyui-models",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
