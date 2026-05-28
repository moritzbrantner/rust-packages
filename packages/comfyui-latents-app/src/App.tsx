import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/comfyui-latents-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "comfyui-latents",
  title: "Comfyui Latents",
  description: "ComfyUI-oriented latent-space data contracts built on tensor-data.",
  domain: "comfyui",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/comfyui-latents",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
