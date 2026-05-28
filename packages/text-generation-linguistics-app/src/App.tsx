import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/text-generation-linguistics-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "text-generation-linguistics",
  title: "Text Generation Linguistics",
  description: "Adapters from text-linguistics analysis outputs into text-generation.",
  domain: "text",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/text-generation-linguistics",
    standaloneRoute: "",
  },
  defaultOperation: "generationLinguistics.synthesizeFromAnalysis",
  featuredOperations: [
    "generationLinguistics.synthesizeFromAnalysis",
    "generationLinguistics.analysisTerms",
    "generationLinguistics.trainAnalysis",
    "describe",
  ],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run linguistic term extraction, synthesis, and Markov training workflows.",
      operations: [
        "generationLinguistics.synthesizeFromAnalysis",
        "generationLinguistics.analysisTerms",
        "generationLinguistics.trainAnalysis",
      ],
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
      id: "synthesize",
      label: "Synthesize",
      operation: "generationLinguistics.synthesizeFromAnalysis",
      description: "Analyze text and synthesize a deterministic document from linguistic terms.",
      input: { id: "analysis-doc", text: "Alice presented the tokenizer roadmap in Berlin." },
    },
    {
      id: "train",
      label: "Train",
      operation: "generationLinguistics.trainAnalysis",
      description: "Train a transient Markov chain from linguistic tokens.",
      input: { text: "Scene transitions follow visual changes.", mode: "lemma", order: 2 },
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
