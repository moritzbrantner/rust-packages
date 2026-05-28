import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/text-transcripts-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "text-transcripts",
  title: "Text Transcripts",
  description: "Transcript parsing and ASR command adapters for video-analysis.",
  domain: "text",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/text-transcripts",
    standaloneRoute: "",
  },
  defaultOperation: "transcripts.parse",
  featuredOperations: ["transcripts.parse", "transcripts.normalize", "transcripts.formatSrt", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run transcript parsing, normalization, and SRT formatting workflows.",
      operations: ["transcripts.parse", "transcripts.normalize", "transcripts.formatSrt"],
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
      id: "parse-srt",
      label: "Parse SRT",
      operation: "transcripts.parse",
      description: "Parse SRT content into the transcript contract.",
      input: { format: "srt", content: "1\n00:00:01,000 --> 00:00:02,000\nHello.\n" },
    },
    {
      id: "format-srt",
      label: "Format SRT",
      operation: "transcripts.formatSrt",
      description: "Format a transcript contract as SRT.",
      input: {
        segments: [{ index: 0, startSeconds: 1.0, endSeconds: 2.0, text: "Hello.", isFinal: true }],
      },
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
