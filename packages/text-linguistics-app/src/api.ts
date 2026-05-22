export interface LinguisticAnalysisSummary {
  language: string | null;
  tokenCount: number;
  sentenceCount: number;
  lemmaCount: number;
  entityCount: number;
  eventCount: number;
  relationCount: number;
  topicCount: number;
  chunkCount: number;
}

export interface LinguisticToken {
  index: number;
  text: string;
  normalized: string;
  kind: string;
  start: number;
  end: number;
}

export interface LinguisticSentence {
  index: number;
  text: string;
  start: number;
  end: number;
  tokenCount: number;
}

export interface LinguisticLemma {
  tokenIndex: number;
  token: string | null;
  lemma: string;
  language: string | null;
  confidence: number;
}

export interface LinguisticPos {
  tokenIndex: number;
  token: string | null;
  tag: string;
  confidence: number;
  reason: string;
}

export interface LinguisticEntity {
  id: string;
  text: string;
  normalized: string;
  kind: string;
  sentenceIndex: number;
  tokenStart: number;
  tokenEnd: number;
  confidence: number;
}

export interface LinguisticEvent {
  sentenceIndex: number;
  predicate: string;
  lemma: string;
  relationType: string;
  confidence: number;
  arguments: Array<{ role: string; text: string; confidence: number }>;
}

export interface LinguisticRelation {
  subject: string;
  relation: string;
  object: string;
  relationType: string;
  confidence: number;
}

export interface LinguisticTopic {
  label: string;
  terms: string[];
  score: number;
}

export interface LinguisticStyle {
  register: string;
  averageSentenceTokens: number;
  typeTokenRatio: number;
  formalityScore: number;
  questionCount: number;
  exclamationCount: number;
}

export interface LinguisticAnalysisPayload {
  package: string;
  library: string;
  accepted: true;
  operation: "analyze";
  text: string;
  profile: string;
  provenance: string;
  confidence: number;
  model?: {
    entityRecognition: string;
    entityModel: string | null;
    tokenizerMode: string;
    tokenizerSource: string | null;
    alignmentCount: number;
  };
  summary: LinguisticAnalysisSummary;
  language: {
    primary: null | {
      language: string;
      confidence: number;
      script: string | null;
      reason: string;
    };
    dominantScript: string | null;
    isMixed: boolean;
    tokenCount: number;
  };
  tokens: LinguisticToken[];
  sentences: LinguisticSentence[];
  lemmas: LinguisticLemma[];
  pos: LinguisticPos[];
  entities: LinguisticEntity[];
  events: LinguisticEvent[];
  relations: LinguisticRelation[];
  topics: LinguisticTopic[];
  style: LinguisticStyle;
}

const configuredServerUrl = import.meta.env.VITE_SERVER_URL as string | undefined;
let wasmInit: Promise<unknown> | null = null;

export const serverBaseUrl = configuredServerUrl ?? "http://127.0.0.1:3000";
export const wrappedLibrary = "text-linguistics";

export async function analyzeLinguistics(text: string): Promise<LinguisticAnalysisPayload> {
  const body = JSON.stringify({ operation: "analyze", profile: "rich", modelMode: "local-model", text });
  const packagePath = `/api/rust/packages/${wrappedLibrary}/api/run`;
  const rootPath = "/api/run";

  try {
    return await postJson<LinguisticAnalysisPayload>(packagePath, body);
  } catch (error) {
    if (!isNotFound(error)) {
      throw error;
    }
    return postJson<LinguisticAnalysisPayload>(rootPath, body);
  }
}

export async function analyzeLinguisticsClient(text: string): Promise<LinguisticAnalysisPayload> {
  wasmInit ??= initTextLinguisticsWasm();
  await wasmInit;
  return analyzeTextLinguistics(text, {
    profile: "balanced",
    entityRecognition: "heuristic",
  }) as LinguisticAnalysisPayload;
}

async function postJson<T>(path: string, body: string): Promise<T> {
  const response = await fetch(`${serverBaseUrl}${path}`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body,
  });
  if (!response.ok) {
    const message = await response.text();
    throw new Error(`Server returned ${response.status}: ${message}`);
  }
  return response.json() as Promise<T>;
}

function isNotFound(error: unknown): boolean {
  return error instanceof Error && error.message.includes("Server returned 404");
}
import initTextLinguisticsWasm, {
  analyzeTextLinguistics,
} from "@mb-rust/text-linguistics-wasm";
