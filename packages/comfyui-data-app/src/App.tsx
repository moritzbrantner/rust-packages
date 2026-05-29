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
  defaultOperation: "comfy.workflow.validate",
  featuredOperations: ["comfy.workflow.validate", "comfy.prompt.links", "comfy.workflow.inventory", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["comfy.workflow.validate", "comfy.prompt.links"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe", "comfy.workflow.inventory"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
