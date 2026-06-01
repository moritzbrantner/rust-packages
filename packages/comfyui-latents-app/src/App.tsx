import { PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/comfyui-latents-wasm";

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
  defaultOperation: "comfy.latents.size",
  featuredOperations: ["comfy.latents.size", "comfy.latents.maskCompatibility", "comfy.latents.batchSummary", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["comfy.latents.size", "comfy.latents.maskCompatibility"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe", "comfy.latents.batchSummary"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
