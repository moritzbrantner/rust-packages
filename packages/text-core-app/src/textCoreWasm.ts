import init, {
  analyzeTextDocument,
  type TextDocumentAnalysis,
  type TextProcessingOptions,
} from "@mb-rust/text-core-wasm";

let initPromise: Promise<void> | null = null;

export type { TextDocumentAnalysis, TextProcessingOptions };

export async function initTextCoreWasm(): Promise<void> {
  initPromise ??= init().then(() => undefined);
  return initPromise;
}

export function analyzeText(text: string, options: TextProcessingOptions): TextDocumentAnalysis {
  try {
    return analyzeTextDocument(text, options);
  } catch (caught) {
    if (caught instanceof Error) {
      throw caught;
    }
    throw new Error(`text-core-wasm analysis failed: ${String(caught)}`);
  }
}
