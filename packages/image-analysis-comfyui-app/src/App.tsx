import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/image-analysis-comfyui-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "image-analysis-comfyui",
  title: "Image Analysis Comfyui",
  description: "ComfyUI workflow builders for image generation and manipulation in video-analysis.",
  domain: "image",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/image-analysis-comfyui",
    standaloneRoute: "",
  },
  defaultOperation: "image.comfyui.promptPlan",
  featuredOperations: [
    "image.comfyui.promptPlan",
    "image.comfyui.workflowSummary",
    "image.comfyui.assetMap",
    "describe",
  ],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Build deterministic ComfyUI prompt graph plans.",
      operations: ["image.comfyui.promptPlan"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect workflow summaries, assets, and package metadata.",
      operations: ["image.comfyui.workflowSummary", "image.comfyui.assetMap", "describe"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
