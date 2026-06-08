import { PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/image-analysis-captioning-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "image-analysis-captioning",
  title: "Image Analysis Captioning",
  description: "Aggregate image task schemas, catalogs, and backend contracts for video-analysis.",
  domain: "image",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/image-analysis-captioning",
    standaloneRoute: "",
  },
  defaultOperation: "image.captioning.imported",
  featuredOperations: [
    "image.captioning.imported",
    "image.captioning.rankCaptions",
    "image.captioning.captionReport",
    "image.captioning.models",
    "image.captioning.schema",
    "describe",
  ],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Validate, rank, and summarize imported caption results.",
      operations: [
        "image.captioning.imported",
        "image.captioning.rankCaptions",
        "image.captioning.captionReport",
      ],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect captioning catalogs, schemas, and package metadata.",
      operations: ["image.captioning.models", "image.captioning.schema", "describe"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
