import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/text-generation-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "text-generation",
  title: "Text Generation",
  description: "Deterministic Markov-chain prediction and text synthesis for video-analysis.",
  domain: "text",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/text-generation",
    standaloneRoute: "",
  },
  defaultOperation: "generation.markovGenerate",
  featuredOperations: ["generation.markovGenerate", "generation.markovPredict", "generation.synthesizeTerms", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run deterministic Markov prediction, generation, and term synthesis workflows.",
      operations: ["generation.markovGenerate", "generation.markovPredict", "generation.synthesizeTerms"],
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
      id: "markov-generate",
      label: "Generate",
      operation: "generation.markovGenerate",
      description: "Train a transient Markov chain and generate text.",
      input: { trainingTexts: ["rust text analysis supports crates"], order: 2, maxTokens: 6 },
    },
    {
      id: "term-synthesis",
      label: "Terms",
      operation: "generation.synthesizeTerms",
      description: "Synthesize deterministic text from weighted terms.",
      input: { terms: [{ term: "rust", weight: 2.0 }, { term: "analysis", weight: 1.0 }] },
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
