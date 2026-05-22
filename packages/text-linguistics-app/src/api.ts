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

export type NlpFallbackPolicy = "error" | "fast_fallback" | "lexical_fallback";

export interface NlpModelSelection {
  modelId?: string;
  runtime?: string;
  fallbackPolicy?: NlpFallbackPolicy;
}

export interface TextClassPrediction {
  label: string;
  score: number;
}

export interface NlpModelMetadata {
  id: string;
  modelId: string;
  task: string;
  runtime: string;
  supported: boolean;
  fallback?: string | null;
  note?: string | null;
}

export interface TextClassificationPayload {
  accepted: true;
  operation: "classify";
  text: string;
  modelId: string;
  runtime: string;
  predictions: TextClassPrediction[];
}

export interface SentimentPayload {
  accepted: true;
  operation: "sentiment";
  text: string;
  modelId: string;
  runtime: string;
  label: string;
  positiveScore: number;
  negativeScore: number;
  compound: number;
  predictions: TextClassPrediction[];
}

export interface EmbeddingPayload {
  accepted: true;
  operation: "embed";
  modelId: string;
  runtime: string;
  dimensions: number;
  embeddings: number[][];
}

export interface ZeroShotPayload {
  accepted: true;
  operation: "zero-shot";
  text: string;
  modelId: string;
  runtime: string;
  predictions: TextClassPrediction[];
  hypotheses: string[];
}

export interface SummaryPayload {
  accepted: true;
  operation: "summarize";
  modelId: string;
  runtime: string;
  strategy: string;
  summary: string;
  sentences: Array<{ index: number; text: string; score: number }>;
}

export interface RerankPayload {
  accepted: true;
  operation: "rerank";
  query: string;
  modelId: string;
  runtime: string;
  results: Array<{ index: number; document: string; score: number }>;
}

export interface QuestionAnswerPayload {
  accepted: true;
  operation: "question-answer";
  question: string;
  modelId: string;
  runtime: string;
  answers: Array<{ answer: string; score: number }>;
}

const configuredServerUrl = import.meta.env.VITE_SERVER_URL as string | undefined;
let wasmInit: Promise<unknown> | null = null;

export const serverBaseUrl = configuredServerUrl ?? "http://127.0.0.1:3000";
export const wrappedLibrary = "text-linguistics";

export async function analyzeLinguistics(text: string): Promise<LinguisticAnalysisPayload> {
  const body = JSON.stringify({
    operation: "analyze",
    profile: "rich",
    modelMode: "local-model",
    text,
  });
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

export async function listNlpModels(task?: string): Promise<NlpModelMetadata[]> {
  return getJson<NlpModelMetadata[]>(task ? `/api/models/${task}` : "/api/models");
}

export async function classifyText(
  text: string,
  labels: string[] = [],
  model?: NlpModelSelection,
): Promise<TextClassificationPayload> {
  return postTaskJson<TextClassificationPayload>("classify", {
    text,
    labels,
    model: { ...model, fallbackPolicy: model?.fallbackPolicy ?? "lexical_fallback" },
  });
}

export async function analyzeSentiment(
  text: string,
  model?: NlpModelSelection,
): Promise<SentimentPayload> {
  return postTaskJson<SentimentPayload>("sentiment", {
    text,
    model: { ...model, fallbackPolicy: model?.fallbackPolicy ?? "lexical_fallback" },
  });
}

export async function embedText(
  texts: string[],
  model?: NlpModelSelection,
): Promise<EmbeddingPayload> {
  return postTaskJson<EmbeddingPayload>("embed", {
    texts,
    model: { ...model, fallbackPolicy: model?.fallbackPolicy ?? "fast_fallback" },
    dimensions: 64,
    normalize: true,
  });
}

export async function zeroShotClassify(
  text: string,
  labels: string[],
  model?: NlpModelSelection,
): Promise<ZeroShotPayload> {
  return postTaskJson<ZeroShotPayload>("zero-shot", {
    text,
    labels,
    model: { ...model, fallbackPolicy: model?.fallbackPolicy ?? "lexical_fallback" },
  });
}

export async function summarizeText(
  text: string,
  model?: NlpModelSelection,
): Promise<SummaryPayload> {
  return postTaskJson<SummaryPayload>("summarize", {
    text,
    maxSentences: 3,
    model,
    strategy: "embedding_extractive",
  });
}

export async function rerankDocuments(
  query: string,
  documents: string[],
  model?: NlpModelSelection,
): Promise<RerankPayload> {
  return postTaskJson<RerankPayload>("rerank", {
    query,
    documents,
    topK: 5,
    model: { ...model, fallbackPolicy: model?.fallbackPolicy ?? "lexical_fallback" },
  });
}

export async function answerQuestion(
  question: string,
  context: string,
  model?: NlpModelSelection,
): Promise<QuestionAnswerPayload> {
  return postTaskJson<QuestionAnswerPayload>("question-answer", {
    question,
    context,
    model,
    importedPredictions: [],
  });
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

async function postTaskJson<T>(taskPath: string, payload: unknown): Promise<T> {
  const body = JSON.stringify(payload);
  const packagePath = `/api/rust/packages/${wrappedLibrary}/api/${taskPath}`;
  const rootPath = `/api/${taskPath}`;

  try {
    return await postJson<T>(packagePath, body);
  } catch (error) {
    if (!isNotFound(error)) {
      throw error;
    }
    return postJson<T>(rootPath, body);
  }
}

async function getJson<T>(path: string): Promise<T> {
  const packagePath = `/api/rust/packages/${wrappedLibrary}${path}`;

  try {
    return await fetchJson<T>(packagePath);
  } catch (error) {
    if (!isNotFound(error)) {
      throw error;
    }
    return fetchJson<T>(path);
  }
}

async function fetchJson<T>(path: string): Promise<T> {
  const response = await fetch(`${serverBaseUrl}${path}`);
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
  postprocessClassification,
  postprocessEmbeddings,
  postprocessSentiment,
  postprocessZeroShot,
  rerankFromImportedScores,
  summarizeLexical,
} from "@mb-rust/text-linguistics-wasm";

export const clientPostprocessors = {
  postprocessClassification,
  postprocessEmbeddings,
  postprocessSentiment,
  postprocessZeroShot,
  rerankFromImportedScores,
  summarizeLexical,
};
