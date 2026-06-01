import { PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/image-analysis-classification-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "image-analysis-classification",
  title: "Image Analysis Classification",
  description: "Aggregate image task schemas, catalogs, and backend contracts for video-analysis.",
  domain: "image",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/image-analysis-classification",
    standaloneRoute: "",
  },
  defaultOperation: "image.classification.imported",
  featuredOperations: [
    "image.classification.imported",
    "image.classification.models",
    "image.classification.schema",
    "describe",
  ],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Validate and normalize imported classification results.",
      operations: ["image.classification.imported"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect classification catalogs, schemas, and package metadata.",
      operations: ["image.classification.models", "image.classification.schema", "describe"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
