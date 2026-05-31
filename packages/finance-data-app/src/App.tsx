import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/finance-data-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "finance-data",
  title: "Finance Data",
  description: "Provider-neutral financial market data validation, indexing, and derived series.",
  domain: "data",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/finance-data",
    standaloneRoute: "",
  },
  defaultOperation: "financeData.bounds",
  featuredOperations: [
    "financeData.bounds",
    "financeData.barsInRange",
    "financeData.downsampleOhlcv",
    "financeData.returns",
    "financeData.riskSummary",
    "describe",
  ],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: [
        "financeData.bounds",
        "financeData.barsInRange",
        "financeData.downsampleOhlcv",
        "financeData.returns",
        "financeData.riskSummary",
      ],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
