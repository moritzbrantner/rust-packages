import { createTextResultTabs, PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/text-model-runtime-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "text-model-runtime",
  title: "Text Model Runtime",
  description: "Shared tokenizer and native text model runtime traits for video-analysis.",
  domain: "text",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/text-model-runtime",
    standaloneRoute: "",
  },
  defaultOperation: "runtime.tokenizeSummary",
  featuredOperations: ["runtime.tokenizeSummary", "runtime.softmax", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run deterministic tokenizer summary workflows.",
      operations: ["runtime.tokenizeSummary"],
    },
    {
      id: "support",
      label: "Support",
      description: "Run reusable runtime support helpers.",
      operations: ["runtime.softmax"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect package metadata and operation support.",
      operations: ["describe"],
    },
  ],
  presets: [
    {
      id: "tokenize-summary",
      label: "Summarize tokenizer offsets",
      operation: "runtime.tokenizeSummary",
      description: "Build a deterministic whitespace-token summary.",
      input: { text: "Rust text runtime summarizes token offsets for transcript package surfaces.", maxTokens: 12 },
    },
    {
      id: "softmax",
      label: "Normalize logits",
      operation: "runtime.softmax",
      description: "Normalize support logits into a probability distribution.",
      input: { logits: [0.1, 0.3, 1.2, -0.7, 2.4, 0.0] },
    },
  ],
  benchmarkScenarios: [
    {
      id: "tokenizer-summary",
      label: "Tokenizer Summary",
      operation: "runtime.tokenizeSummary",
      input: { text: "Rust text runtime summarizes token offsets for package surfaces.", maxTokens: 12 },
      iterations: 120,
      warmupIterations: 5,
      outputCountPath: ["tokens"],
    },
    {
      id: "softmax",
      label: "Softmax",
      operation: "runtime.softmax",
      input: { logits: [0.1, 0.3, 1.2, -0.7, 2.4, 0.0] },
      iterations: 200,
      warmupIterations: 10,
      outputCountPath: ["probabilities"],
    },
  ],
  resultTabs: createTextResultTabs({
    library: "text-model-runtime",
    primaryOperations: {
      "runtime.tokenizeSummary": {
        title: "Tokenizer summary",
        summaryFields: ["tokenCount", "maxTokens", "truncated"],
        listFields: ["tokens"],
        objectFields: ["offsets", "metadata", "result"],
        explanation: () => "The runtime helper split text deterministically and reported token offsets/counts without invoking an optional native model runtime.",
      },
      "runtime.softmax": {
        title: "Softmax probabilities",
        summaryFields: ["probabilityCount", "maxProbability"],
        listFields: ["probabilities"],
        objectFields: ["result"],
        explanation: () => "The support helper normalized logits with a stable softmax pass so downstream model wrappers can present probabilities.",
      },
    },
  }),
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
