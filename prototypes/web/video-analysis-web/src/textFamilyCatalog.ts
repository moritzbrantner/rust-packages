export interface TextFamilyEntry {
  library: string;
  label: string;
  description: string;
  presetId?: string;
  operationId?: string;
  badges: string[];
}

export interface TextFamilyTier {
  id: string;
  label: string;
  description: string;
  primary?: boolean;
  collapsedByDefault?: boolean;
  entries: TextFamilyEntry[];
}

export const textFamilyTiers: TextFamilyTier[] = [
  {
    id: "analyze",
    label: "Analyze",
    description: "Start with composed document, corpus, and similarity reports from text-analysis.",
    primary: true,
    entries: [
      {
        library: "text-analysis",
        label: "Text Analysis",
        description: "Unified deterministic document analysis with core stats, lexical features, linguistic hints, and embedding diagnostics.",
        presetId: "document-deterministic",
        operationId: "analysis.document",
        badges: ["Primary", "Document report"],
      },
      {
        library: "text-analysis",
        label: "Corpus Analysis",
        description: "Transient corpus search, near-duplicate checks, and semantic-neighbor analysis for transcript-style collections.",
        presetId: "corpus",
        operationId: "analysis.corpus",
        badges: ["Secondary", "Corpus"],
      },
      {
        library: "text-analysis",
        label: "Similarity Analysis",
        description: "Compare two passages with deterministic overlap scoring for package-consumer smoke tests and workflow probes.",
        presetId: "similarity",
        operationId: "analysis.similarity",
        badges: ["Secondary", "Similarity"],
      },
    ],
  },
  {
    id: "search",
    label: "Search",
    description: "Build searchable text indexes first, then use embeddings when semantic-only search is the better fit.",
    entries: [
      {
        library: "text-index",
        label: "Text Index",
        description: "Durable-oriented hybrid indexing, chunk metadata, facets, and transient browser-safe search.",
        presetId: "hybrid-search",
        operationId: "index.search",
        badges: ["Recommended search", "Hybrid"],
      },
      {
        library: "text-embeddings",
        label: "Text Embeddings",
        description: "Deterministic embeddings, similarity, semantic search, and related-term workflows.",
        presetId: "semantic-search",
        operationId: "embeddings.semanticSearch",
        badges: ["Semantic search"],
      },
    ],
  },
  {
    id: "task-apis",
    label: "Task APIs",
    description: "Focused workbenches for package consumers adding classification, QA, or generation surfaces.",
    entries: [
      {
        library: "text-classification",
        label: "Text Classification",
        description: "Classification request schemas, deterministic lexical fallback, sentiment, and zero-shot-compatible surfaces.",
        presetId: "classify",
        operationId: "classification.classify",
        badges: ["Classification"],
      },
      {
        library: "text-question-answering",
        label: "Text Question Answering",
        description: "Question answering over direct context, text-index citations, retrieval compatibility, and batches.",
        presetId: "index-answer",
        operationId: "qa.answerWithIndex",
        badges: ["QA", "Index-backed"],
      },
      {
        library: "text-generation",
        label: "Text Generation",
        description: "Deterministic Markov generation, prediction, perplexity, and term synthesis without model downloads.",
        presetId: "markov-generate",
        operationId: "generation.markovGenerate",
        badges: ["Generation"],
      },
    ],
  },
  {
    id: "foundations",
    label: "Foundations",
    description: "Lower-level text building blocks for tokenization, lexical features, linguistics, and transcript adapters.",
    entries: [
      {
        library: "text-core",
        label: "Text Core",
        description: "Core normalization, tokenization, statistics, and boundary detection contracts.",
        presetId: "tokenize-transcript-notes",
        operationId: "text.tokenize",
        badges: ["Core"],
      },
      {
        library: "text-lexical",
        label: "Text Lexical",
        description: "Lexical analysis, keyword extraction, transient corpus search, and corpus statistics.",
        presetId: "lexical-analysis",
        operationId: "lexical.analyze",
        badges: ["Lexical"],
      },
      {
        library: "text-linguistics",
        label: "Text Linguistics",
        description: "Heuristic language, entity, and linguistic analysis projections for deterministic workflows.",
        presetId: "balanced-analysis",
        operationId: "linguistics.analyze",
        badges: ["Linguistics"],
      },
      {
        library: "text-transcripts",
        label: "Text Transcripts",
        description: "Transcript parsing, normalization, segment conversion, and subtitle formatting adapters.",
        presetId: "to-text-segments",
        operationId: "transcripts.toTextSegments",
        badges: ["Transcripts"],
      },
    ],
  },
  {
    id: "runtime-setup",
    label: "Runtime Setup",
    description: "Model-runtime metadata and bundle checks remain explicit setup surfaces for package consumers.",
    entries: [
      {
        library: "text-model-runtime",
        label: "Text Model Runtime",
        description: "Bundle checks, tokenizer probes, and runtime diagnostics for opt-in model-backed text workflows.",
        presetId: "bundle-check",
        operationId: "runtime.bundleCheck",
        badges: ["Runtime", "Setup"],
      },
    ],
  },
  {
    id: "compatibility",
    label: "Compatibility And Adapters",
    description: "Soft-legacy and adapter surfaces stay reachable without competing with primary text workflows.",
    collapsedByDefault: true,
    entries: [
      {
        library: "text-retrieval",
        label: "Text Retrieval",
        description: "Soft-legacy RetrievalIndex compatibility, reranking, older search document adapters, and snapshot planning.",
        presetId: "hybrid-search",
        operationId: "retrieval.search",
        badges: ["Compatibility", "Hybrid"],
      },
      {
        library: "text-generation-linguistics",
        label: "Text Generation Linguistics",
        description: "Adapter workflows that synthesize deterministic text from linguistic analysis terms.",
        presetId: "synthesize",
        operationId: "generationLinguistics.synthesizeFromAnalysis",
        badges: ["Adapter"],
      },
    ],
  },
];

export function textFamilyEntryHref(entry: TextFamilyEntry, baseHref = "/"): string {
  const normalizedBase = baseHref.endsWith("/") ? baseHref : `${baseHref}/`;
  const preset = entry.presetId ? `?preset=${encodeURIComponent(entry.presetId)}` : "";
  return `${normalizedBase}wrappers/${encodeURIComponent(entry.library)}/${preset}`;
}
