import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/image-analysis-ocr-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "image-analysis-ocr",
  title: "Image Analysis OCR",
  description: "OCR model presets, rich text outputs, and image/video backend contracts for video-analysis.",
  domain: "image",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/image-analysis-ocr",
    standaloneRoute: "",
  },
  defaultOperation: "image.ocr.documentSummary",
  featuredOperations: ["image.ocr.documentSummary", "image.ocr.requestSummary", "image.ocr.presets", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Summarize imported OCR document results.",
      operations: ["image.ocr.documentSummary"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect OCR presets, request options, and package metadata.",
      operations: ["image.ocr.presets", "image.ocr.requestSummary", "describe"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
