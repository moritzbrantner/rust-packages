import { PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/math-statistics-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "math-statistics",
  title: "Math Statistics",
  description: "Shared scalar, pairwise, rolling, multivariate, and matrix statistics.",
  domain: "math",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/math-statistics",
    standaloneRoute: "",
  },
  defaultOperation: "stats.series.describe",
  featuredOperations: [
    "stats.series.describe",
    "stats.series.changes",
    "stats.series.compare",
    "stats.series.rolling",
    "stats.series.tailRisk",
    "stats.series.zScores",
    "stats.normalize",
    "stats.covariance",
    "stats.pca",
    "describe",
  ],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: [
        "stats.series.describe",
        "stats.series.changes",
        "stats.series.compare",
        "stats.series.rolling",
        "stats.series.tailRisk",
        "stats.series.zScores",
        "stats.normalize",
        "stats.covariance",
        "stats.pca",
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
