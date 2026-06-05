import { PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/finance-statistics-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "finance-statistics",
  title: "Finance Statistics",
  description: "Finance-oriented return, risk, and rolling statistics helpers.",
  domain: "math",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/finance-statistics",
    standaloneRoute: "",
  },
  defaultOperation: "finance.returns",
  featuredOperations: [
    "finance.returns",
    "finance.risk",
    "finance.drawdown",
    "finance.rolling",
    "finance.portfolio",
    "finance.performanceRatios",
    "describe",
  ],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: [
        "finance.returns",
        "finance.risk",
        "finance.drawdown",
        "finance.rolling",
        "finance.portfolio",
        "finance.performanceRatios",
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
