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
  defaultOperation: "comfy.models.defaults",
  featuredOperations: ["comfy.models.defaults", "comfy.models.reference", "comfy.models.extraPathsPlan", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["comfy.models.defaults", "comfy.models.reference"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe", "comfy.models.extraPathsPlan"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
