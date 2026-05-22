import { FormEvent, useEffect, useMemo, useState, type ReactNode } from "react";

import {
  analyzeSentiment,
  answerQuestion,
  analyzeLinguistics,
  analyzeLinguisticsClient,
  classifyText,
  embedText,
  listNlpModels,
  rerankDocuments,
  serverBaseUrl,
  summarizeText,
  type EmbeddingPayload,
  type QuestionAnswerPayload,
  type RerankPayload,
  type SentimentPayload,
  type SummaryPayload,
  type TextClassificationPayload,
  type LinguisticAnalysisPayload,
  type LinguisticEntity,
  type LinguisticEvent,
  type LinguisticLemma,
  type LinguisticPos,
  type LinguisticRelation,
  type LinguisticSentence,
  type LinguisticToken,
  type LinguisticTopic,
  type NlpModelMetadata,
  type ZeroShotPayload,
  zeroShotClassify,
} from "./api";
import { sampleText } from "./sampleText";

type LoadState = "idle" | "loading" | "ready" | "error";
type RuntimeMode = "server" | "client-wasm";
type NlpTask =
  | "entities"
  | "sentiment"
  | "classify"
  | "embed"
  | "zero-shot"
  | "summarize"
  | "rerank"
  | "qa";
type TaskResultPayload =
  | SentimentPayload
  | TextClassificationPayload
  | EmbeddingPayload
  | ZeroShotPayload
  | SummaryPayload
  | RerankPayload
  | QuestionAnswerPayload;

const nlpTasks: Array<{ id: NlpTask; label: string; purpose: string }> = [
  {
    id: "entities",
    label: "Linguistics",
    purpose:
      "Runs the full local analysis pipeline: language, tokens, syntax, entities, events, topics, and style.",
  },
  {
    id: "sentiment",
    label: "Sentiment",
    purpose: "Scores whether the text reads positive, negative, or neutral.",
  },
  {
    id: "classify",
    label: "Classify",
    purpose: "Assigns the text to the configured labels.",
  },
  {
    id: "embed",
    label: "Embed",
    purpose: "Creates numeric vectors for similarity search and retrieval.",
  },
  {
    id: "zero-shot",
    label: "Zero-shot",
    purpose: "Ranks labels without task-specific training examples.",
  },
  {
    id: "summarize",
    label: "Summarize",
    purpose: "Extracts the highest-value sentences into a shorter summary.",
  },
  {
    id: "rerank",
    label: "Rerank",
    purpose: "Sorts candidate documents against the first line as the query.",
  },
  {
    id: "qa",
    label: "QA",
    purpose: "Answers the first line as a question against the remaining text.",
  },
];

const taskCatalogKeys: Record<NlpTask, string> = {
  entities: "token_classification",
  sentiment: "sentiment",
  classify: "text_classification",
  embed: "embedding",
  "zero-shot": "zero_shot_classification",
  summarize: "summarization",
  rerank: "reranking",
  qa: "question_answering",
};

const fallbackModelIds: Record<NlpTask, string> = {
  entities: "bert-base-ner",
  sentiment: "twitter-roberta-sentiment-latest",
  classify: "distilbert-sst2",
  embed: "all-mpnet-base-v2",
  "zero-shot": "bart-large-mnli",
  summarize: "embedding-extractive-summary",
  rerank: "ms-marco-minilm-l6-v2",
  qa: "roberta-base-squad2",
};

const panelClass = "min-w-0 rounded-md border border-zinc-200 bg-white p-5 shadow-sm";
const buttonPrimaryClass =
  "rounded-md bg-teal-700 px-3 py-2 text-sm font-semibold text-white transition hover:bg-teal-800 focus:outline-none focus:ring-2 focus:ring-teal-600 focus:ring-offset-2 disabled:cursor-not-allowed disabled:bg-zinc-300";
const buttonSecondaryClass =
  "rounded-md border border-zinc-300 bg-white px-3 py-2 text-sm font-medium text-zinc-800 transition hover:bg-zinc-100 focus:outline-none focus:ring-2 focus:ring-teal-600 focus:ring-offset-2";
const tabButtonClass =
  "min-h-9 rounded-md border border-zinc-200 bg-white px-3 text-sm font-medium text-zinc-700 transition hover:border-zinc-300 hover:bg-zinc-50";
const tabActiveClass =
  "border-zinc-950 bg-zinc-950 text-white hover:border-zinc-950 hover:bg-zinc-950";
const tableWrapClass = "max-h-80 overflow-auto rounded-md border border-zinc-200";
const dataTableClass =
  "w-full border-collapse text-left text-sm [&_tbody_tr:hover]:bg-zinc-50 [&_td]:max-w-xs [&_td]:border-t [&_td]:border-zinc-200 [&_td]:px-3 [&_td]:py-2 [&_td]:align-top [&_td]:text-zinc-900 [&_th]:sticky [&_th]:top-0 [&_th]:bg-zinc-100 [&_th]:px-3 [&_th]:py-2 [&_th]:font-semibold [&_th]:text-zinc-700";
const detailGridClass =
  "grid gap-3 text-sm sm:grid-cols-2 xl:grid-cols-4 [&_dd]:mt-1 [&_dd]:break-words [&_dd]:font-mono [&_dd]:text-zinc-900 [&_div]:min-w-0 [&_div]:rounded-md [&_div]:border [&_div]:border-zinc-200 [&_div]:bg-zinc-50 [&_div]:p-3 [&_dt]:text-xs [&_dt]:font-semibold [&_dt]:text-zinc-500";

export function App() {
  const [text, setText] = useState(sampleText);
  const [analysis, setAnalysis] = useState<LinguisticAnalysisPayload | null>(null);
  const [taskResult, setTaskResult] = useState<TaskResultPayload | null>(null);
  const [loadState, setLoadState] = useState<LoadState>("idle");
  const [runtimeMode, setRuntimeMode] = useState<RuntimeMode>("server");
  const [nlpTask, setNlpTask] = useState<NlpTask>("entities");
  const [modelCatalog, setModelCatalog] = useState<NlpModelMetadata[]>([]);
  const [selectedModelIds, setSelectedModelIds] = useState<Partial<Record<NlpTask, string>>>({});
  const [modelCatalogError, setModelCatalogError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function loadModels() {
      try {
        const models = await listNlpModels();
        if (!cancelled) {
          setModelCatalog(models);
          setModelCatalogError(null);
        }
      } catch (caught) {
        if (!cancelled) {
          setModelCatalog([]);
          setModelCatalogError(
            caught instanceof Error ? caught.message : "Unable to load model catalog",
          );
        }
      }
    }

    loadModels();
    return () => {
      cancelled = true;
    };
  }, []);

  const modelOptions = useMemo(
    () => modelCatalog.filter((model) => model.task === taskCatalogKeys[nlpTask]),
    [modelCatalog, nlpTask],
  );
  const selectedTask = nlpTasks.find((task) => task.id === nlpTask) ?? nlpTasks[0];
  const selectedModelId =
    selectedModelIds[nlpTask] ?? modelOptions[0]?.id ?? fallbackModelIds[nlpTask];
  const selectedModel = modelOptions.find((model) => model.id === selectedModelId);
  const currentModel = describeCurrentModel({
    analysis,
    fallbackModelId: selectedModelId,
    model: selectedModel,
    nlpTask,
    runtimeMode,
    taskResult,
  });

  const json = useMemo(
    () =>
      analysis
        ? JSON.stringify(analysis, null, 2)
        : taskResult
          ? JSON.stringify(taskResult, null, 2)
          : "",
    [analysis, taskResult],
  );
  const statusLabel =
    loadState === "ready"
      ? "Ready"
      : loadState === "loading"
        ? "Analyzing"
        : loadState === "error"
          ? "Error"
          : "Idle";

  async function submit(event?: FormEvent<HTMLFormElement>) {
    event?.preventDefault();
    if (!text.trim()) {
      setError("Enter text before running linguistic analysis.");
      setLoadState("error");
      return;
    }
    setLoadState("loading");
    setError(null);
    try {
      if (nlpTask === "entities") {
        const payload =
          runtimeMode === "server"
            ? await analyzeLinguistics(text)
            : await analyzeLinguisticsClient(text);
        setAnalysis(payload);
        setTaskResult(null);
      } else {
        const payload = await runNlpTask(nlpTask, text, selectedModelId);
        setAnalysis(null);
        setTaskResult(payload);
      }
      setLoadState("ready");
    } catch (caught) {
      setLoadState("error");
      setError(caught instanceof Error ? caught.message : "Analysis failed");
    }
  }

  async function copyJson() {
    if (!json) {
      return;
    }
    try {
      await navigator.clipboard.writeText(json);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Clipboard write failed");
    }
  }

  return (
    <main className="min-h-screen bg-zinc-50 text-zinc-950">
      <header className="border-b border-zinc-200 bg-white">
        <div className="mx-auto flex max-w-7xl flex-col gap-4 px-5 py-4 lg:flex-row lg:items-center lg:justify-between">
          <div>
            <div className="text-sm font-semibold text-teal-700">Package app</div>
            <h1 className="mt-1 text-2xl font-semibold">Text Linguistics</h1>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <span
              className={classNames(
                "inline-flex min-h-9 min-w-24 items-center justify-center rounded-md px-3 text-sm font-semibold",
                loadState === "ready"
                  ? "bg-emerald-100 text-emerald-800"
                  : loadState === "loading"
                    ? "bg-amber-100 text-amber-800"
                    : loadState === "error"
                      ? "bg-rose-100 text-rose-800"
                      : "bg-zinc-100 text-zinc-700",
              )}
            >
              {statusLabel}
            </span>
            <button
              className={buttonSecondaryClass}
              type="button"
              onClick={() => setText(sampleText)}
            >
              Reset sample
            </button>
            <button className={buttonSecondaryClass} type="button" onClick={() => setText("")}>
              Clear
            </button>
            <button
              className={buttonSecondaryClass}
              type="button"
              disabled={!json}
              onClick={copyJson}
            >
              Copy JSON
            </button>
          </div>
        </div>
      </header>

      <section className="mx-auto grid w-full max-w-screen-2xl gap-5 px-5 py-5 xl:grid-cols-[minmax(360px,0.8fr)_minmax(0,1.2fr)]">
        <form className={panelClass} onSubmit={submit}>
          <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
            <div>
              <h2 className="text-base font-semibold text-zinc-950">Input</h2>
              <p className="mt-1 text-sm text-zinc-500">
                {nlpTask === "entities"
                  ? runtimeMode === "server"
                    ? serverBaseUrl
                    : "Client WASM"
                  : serverBaseUrl}
              </p>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              {nlpTasks.map((task) => (
                <RuntimeButton
                  key={task.id}
                  active={nlpTask === task.id}
                  onClick={() => setNlpTask(task.id)}
                >
                  {task.label}
                </RuntimeButton>
              ))}
            </div>
            <div className="mt-3 flex flex-wrap items-center gap-2">
              {nlpTask === "entities" ? (
                <>
                  <RuntimeButton
                    active={runtimeMode === "server"}
                    onClick={() => setRuntimeMode("server")}
                  >
                    Server
                  </RuntimeButton>
                  <RuntimeButton
                    active={runtimeMode === "client-wasm"}
                    onClick={() => setRuntimeMode("client-wasm")}
                  >
                    Client WASM
                  </RuntimeButton>
                </>
              ) : null}
              <button
                className={buttonPrimaryClass}
                type="submit"
                disabled={loadState === "loading"}
              >
                {nlpTask === "entities" ? "Analyze" : "Run"}
              </button>
            </div>
          </div>
          <p className="mt-4 rounded-md border border-zinc-200 bg-zinc-50 px-3 py-2 text-sm text-zinc-600">
            {selectedTask.purpose}
          </p>
          <div className="mt-4 grid gap-3 rounded-md border border-zinc-200 bg-zinc-50 p-3 text-sm md:grid-cols-[minmax(0,1fr)_minmax(220px,320px)]">
            <div className="min-w-0">
              <div className="text-xs font-semibold uppercase text-zinc-500">Current model</div>
              <div className="mt-1 truncate font-mono text-base font-semibold text-zinc-950">
                {currentModel.label}
              </div>
              <div className="mt-1 truncate text-xs text-zinc-500">{currentModel.detail}</div>
            </div>
            <div className="min-w-0">
              <label
                className="text-xs font-semibold uppercase text-zinc-500"
                htmlFor="model-select"
              >
                Model preset
              </label>
              {modelOptions.length > 1 ? (
                <select
                  id="model-select"
                  className="mt-1 h-10 w-full rounded-md border border-zinc-300 bg-white px-3 text-sm text-zinc-950 outline-none focus:border-teal-500 focus:ring-2 focus:ring-teal-200"
                  value={selectedModelId}
                  onChange={(event) =>
                    setSelectedModelIds((values) => ({ ...values, [nlpTask]: event.target.value }))
                  }
                >
                  {modelOptions.map((model) => (
                    <option key={model.id} value={model.id}>
                      {model.id} ({formatRuntime(model.runtime)})
                    </option>
                  ))}
                </select>
              ) : (
                <div className="mt-1 flex min-h-10 items-center rounded-md border border-zinc-200 bg-white px-3 font-mono text-sm text-zinc-900">
                  {currentModel.label}
                </div>
              )}
              {modelCatalogError ? (
                <div className="mt-1 text-xs text-amber-700">
                  Catalog unavailable; using defaults.
                </div>
              ) : null}
            </div>
          </div>
          <textarea
            className="mt-4 min-h-64 w-full resize-y rounded-md border border-zinc-300 bg-zinc-950 p-4 font-mono text-sm leading-6 text-zinc-50 outline-none focus:border-teal-500 focus:ring-2 focus:ring-teal-200"
            spellCheck={false}
            value={text}
            onChange={(event) => setText(event.target.value)}
          />
          {error ? (
            <p className="mt-4 rounded-md border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-800">
              {error}
            </p>
          ) : null}
        </form>

        <section className={panelClass}>
          <div className="flex flex-col gap-5">
            <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
              <div>
                <h2 className="text-base font-semibold text-zinc-950">Results</h2>
                <p className="mt-1 text-sm text-zinc-500">{selectedTask.purpose}</p>
              </div>
              <button
                className={buttonSecondaryClass}
                type="button"
                disabled={!json}
                onClick={copyJson}
              >
                Copy JSON
              </button>
            </div>
            {analysis ? (
              <LinguisticResults analysis={analysis} json={json} onCopyJson={copyJson} />
            ) : taskResult ? (
              <TaskResultPanel result={taskResult} json={json} onCopyJson={copyJson} />
            ) : (
              <div className="flex min-h-80 items-center justify-center rounded-md border border-dashed border-zinc-300 bg-zinc-50 text-sm font-medium text-zinc-500">
                Run {selectedTask.label} to populate results.
              </div>
            )}
          </div>
        </section>
      </section>
    </main>
  );
}

async function runNlpTask(
  task: Exclude<NlpTask, "entities">,
  text: string,
  modelId: string,
): Promise<TaskResultPayload> {
  const model = { modelId };
  if (task === "sentiment") {
    return analyzeSentiment(text, model);
  }
  if (task === "classify") {
    return classifyText(text, ["technology", "business", "science"], model);
  }
  if (task === "embed") {
    return embedText([text], model);
  }
  if (task === "zero-shot") {
    return zeroShotClassify(text, ["technology", "business", "science", "culture"], model);
  }
  if (task === "summarize") {
    return summarizeText(text, model);
  }
  if (task === "rerank") {
    const lines = text
      .split(/\n+/)
      .map((line) => line.trim())
      .filter(Boolean);
    return rerankDocuments(
      lines[0] ?? text,
      lines.slice(1).length ? lines.slice(1) : [text],
      model,
    );
  }
  const lines = text
    .split(/\n+/)
    .map((line) => line.trim())
    .filter(Boolean);
  return answerQuestion(
    lines[0] ?? "What is this text about?",
    lines.slice(1).join("\n") || text,
    model,
  );
}

function LinguisticResults({
  analysis,
  json,
  onCopyJson,
}: {
  analysis: LinguisticAnalysisPayload;
  json: string;
  onCopyJson: () => void;
}) {
  return (
    <div className="grid gap-5">
      <ResultSection
        title="Overview"
        task="Language and quality"
        description="Top-level counts, confidence, runtime provenance, language, and style signals."
      >
        <Overview analysis={analysis} />
      </ResultSection>
      <ResultSection
        title="Tokens"
        task="Segmentation"
        description="The normalized word and punctuation units that later tasks consume."
      >
        <TokensTable tokens={analysis.tokens} />
      </ResultSection>
      <ResultSection
        title="Syntax"
        task="Sentence, lemma, and POS analysis"
        description="Sentence boundaries, base word forms, and part-of-speech tags for grammar-aware processing."
      >
        <SyntaxTables lemmas={analysis.lemmas} pos={analysis.pos} sentences={analysis.sentences} />
      </ResultSection>
      <ResultSection
        title="Entities"
        task="Named entity recognition"
        description="People, organizations, places, and other named spans for linking or filtering."
      >
        <EntitiesTable entities={analysis.entities} />
      </ResultSection>
      <ResultSection
        title="Events"
        task="Predicate and relation extraction"
        description="Actions or changes found in sentences, with arguments such as who did what to whom. This is useful for timelines, summaries, and downstream event prompts."
      >
        <EventsTable events={analysis.events} relations={analysis.relations} />
      </ResultSection>
      <ResultSection
        title="Topics"
        task="Topic and keyword extraction"
        description="Weighted subject labels from recurring terms. This is useful for overview tags, search facets, routing, and clustering."
      >
        <TopicsPanel topics={analysis.topics} analysis={analysis} />
      </ResultSection>
      <ResultSection
        title="JSON"
        task="Raw response"
        description="Complete payload for debugging, tests, or API integration."
      >
        <JsonPanel json={json} onCopy={onCopyJson} />
      </ResultSection>
    </div>
  );
}

function RuntimeButton({
  active,
  children,
  onClick,
}: {
  active: boolean;
  children: ReactNode;
  onClick: () => void;
}) {
  return (
    <button
      className={classNames(tabButtonClass, active ? tabActiveClass : "")}
      type="button"
      onClick={onClick}
    >
      {children}
    </button>
  );
}

function ResultSection({
  children,
  description,
  task,
  title,
}: {
  children: ReactNode;
  description: string;
  task: string;
  title: string;
}) {
  return (
    <section className="border-t border-zinc-200 pt-5 first:border-t-0 first:pt-0">
      <div className="mb-3">
        <div className="text-xs font-semibold uppercase text-teal-700">{task}</div>
        <h3 className="mt-1 text-base font-semibold text-zinc-950">{title}</h3>
        <p className="mt-1 text-sm text-zinc-500">{description}</p>
      </div>
      {children}
    </section>
  );
}

function TaskResultPanel({
  json,
  onCopyJson,
  result,
}: {
  json: string;
  onCopyJson: () => void;
  result: TaskResultPayload;
}) {
  return (
    <div className="grid gap-5">
      {renderTaskResult(result)}
      <ResultSection
        title="JSON"
        task="Raw response"
        description="Complete payload for debugging, tests, or API integration."
      >
        <JsonPanel json={json} onCopy={onCopyJson} />
      </ResultSection>
    </div>
  );
}

function renderTaskResult(result: TaskResultPayload): ReactNode {
  if (result.operation === "sentiment") {
    return (
      <ResultSection
        title="Sentiment"
        task="Sentiment analysis"
        description="Overall polarity and class scores for tone-sensitive routing or review queues."
      >
        <div className="grid gap-4">
          <dl className={detailGridClass}>
            <div>
              <dt>Label</dt>
              <dd>{result.label}</dd>
            </div>
            <div>
              <dt>Positive</dt>
              <dd>{formatNumber(result.positiveScore)}</dd>
            </div>
            <div>
              <dt>Negative</dt>
              <dd>{formatNumber(result.negativeScore)}</dd>
            </div>
            <div>
              <dt>Compound</dt>
              <dd>{formatNumber(result.compound)}</dd>
            </div>
          </dl>
          <PredictionsTable predictions={result.predictions} />
        </div>
      </ResultSection>
    );
  }

  if (result.operation === "classify") {
    return (
      <ResultSection
        title="Classification"
        task="Text classification"
        description="Scores the text against the configured label set."
      >
        <PredictionsTable predictions={result.predictions} />
      </ResultSection>
    );
  }

  if (result.operation === "zero-shot") {
    return (
      <ResultSection
        title="Zero-shot labels"
        task="Zero-shot classification"
        description="Ranks candidate labels without training a classifier for this app."
      >
        <div className="grid gap-4">
          <PredictionsTable predictions={result.predictions} />
          <dl className={detailGridClass}>
            <div>
              <dt>Hypotheses</dt>
              <dd>{result.hypotheses.join(", ")}</dd>
            </div>
          </dl>
        </div>
      </ResultSection>
    );
  }

  if (result.operation === "embed") {
    return (
      <ResultSection
        title="Embeddings"
        task="Vector embedding"
        description="Numeric vectors for semantic search, clustering, and retrieval pipelines."
      >
        <Table empty={result.embeddings.length === 0 ? "No embeddings returned." : undefined}>
          <thead>
            <tr>
              <th>#</th>
              <th>Dimensions</th>
              <th>Preview</th>
            </tr>
          </thead>
          <tbody>
            {result.embeddings.map((embedding, index) => (
              <tr key={index}>
                <td>{index}</td>
                <td>{embedding.length}</td>
                <td>{embedding.slice(0, 12).map(formatNumber).join(", ")}</td>
              </tr>
            ))}
          </tbody>
        </Table>
      </ResultSection>
    );
  }

  if (result.operation === "summarize") {
    return (
      <ResultSection
        title="Summary"
        task="Extractive summarization"
        description="A compact version of the input plus the source sentences that contributed to it."
      >
        <div className="grid gap-4">
          <p className="rounded-md border border-zinc-200 bg-zinc-50 p-3 text-sm leading-6 text-zinc-900">
            {result.summary}
          </p>
          <Table
            empty={result.sentences.length === 0 ? "No source sentences returned." : undefined}
          >
            <thead>
              <tr>
                <th>Sentence</th>
                <th>Score</th>
                <th>Text</th>
              </tr>
            </thead>
            <tbody>
              {result.sentences.map((sentence) => (
                <tr key={sentence.index}>
                  <td>{sentence.index}</td>
                  <td>{formatNumber(sentence.score)}</td>
                  <td>{sentence.text}</td>
                </tr>
              ))}
            </tbody>
          </Table>
        </div>
      </ResultSection>
    );
  }

  if (result.operation === "rerank") {
    return (
      <ResultSection
        title="Ranked documents"
        task="Reranking"
        description="Candidate document order for retrieval systems, using the first input line as the query."
      >
        <Table empty={result.results.length === 0 ? "No ranked documents returned." : undefined}>
          <thead>
            <tr>
              <th>Rank</th>
              <th>Document</th>
              <th>Score</th>
            </tr>
          </thead>
          <tbody>
            {result.results.map((item, rank) => (
              <tr key={`${item.index}-${rank}`}>
                <td>{rank + 1}</td>
                <td>{item.document}</td>
                <td>{formatNumber(item.score)}</td>
              </tr>
            ))}
          </tbody>
        </Table>
      </ResultSection>
    );
  }

  return (
    <ResultSection
      title="Answers"
      task="Question answering"
      description="Answer spans scored against the supplied context."
    >
      <Table empty={result.answers.length === 0 ? "No answers returned." : undefined}>
        <thead>
          <tr>
            <th>Answer</th>
            <th>Score</th>
          </tr>
        </thead>
        <tbody>
          {result.answers.map((answer, index) => (
            <tr key={`${answer.answer}-${index}`}>
              <td>{answer.answer}</td>
              <td>{formatNumber(answer.score)}</td>
            </tr>
          ))}
        </tbody>
      </Table>
    </ResultSection>
  );
}

function PredictionsTable({
  predictions,
}: {
  predictions: Array<{ label: string; score: number }>;
}) {
  return (
    <Table empty={predictions.length === 0 ? "No predictions returned." : undefined}>
      <thead>
        <tr>
          <th>Label</th>
          <th>Score</th>
        </tr>
      </thead>
      <tbody>
        {predictions.map((prediction) => (
          <tr key={prediction.label}>
            <td>{prediction.label}</td>
            <td>{formatNumber(prediction.score)}</td>
          </tr>
        ))}
      </tbody>
    </Table>
  );
}

function Overview({ analysis }: { analysis: LinguisticAnalysisPayload }) {
  const metrics = [
    ["Language", analysis.summary.language ?? "Unknown"],
    ["Tokens", analysis.summary.tokenCount],
    ["Sentences", analysis.summary.sentenceCount],
    ["Entities", analysis.summary.entityCount],
    ["Events", analysis.summary.eventCount],
    ["Relations", analysis.summary.relationCount],
    ["Topics", analysis.summary.topicCount],
    ["Confidence", formatNumber(analysis.confidence)],
    ["NER", analysis.model?.entityModel ?? analysis.model?.entityRecognition ?? "Unknown"],
  ];

  return (
    <div className="space-y-4">
      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        {metrics.map(([label, value]) => (
          <div key={label} className="rounded-md border border-zinc-200 bg-zinc-50 p-3">
            <div className="text-xs font-semibold text-zinc-500">{label}</div>
            <div className="mt-1 truncate text-xl font-semibold text-zinc-950">{value}</div>
          </div>
        ))}
      </div>
      <dl className={detailGridClass}>
        <div>
          <dt>Profile</dt>
          <dd>{analysis.profile}</dd>
        </div>
        <div>
          <dt>Provenance</dt>
          <dd>{analysis.provenance}</dd>
        </div>
        <div>
          <dt>Entity backend</dt>
          <dd>{analysis.model?.entityRecognition ?? "Unknown"}</dd>
        </div>
        <div>
          <dt>Tokenizer</dt>
          <dd>{analysis.model?.tokenizerMode ?? "Unknown"}</dd>
        </div>
        <div>
          <dt>Aligned tokens</dt>
          <dd>{analysis.model?.alignmentCount ?? 0}</dd>
        </div>
        <div>
          <dt>Script</dt>
          <dd>{analysis.language.dominantScript ?? "None"}</dd>
        </div>
        <div>
          <dt>Register</dt>
          <dd>{analysis.style.register}</dd>
        </div>
        <div>
          <dt>Type-token ratio</dt>
          <dd>{formatNumber(analysis.style.typeTokenRatio)}</dd>
        </div>
        <div>
          <dt>Avg sentence tokens</dt>
          <dd>{formatNumber(analysis.style.averageSentenceTokens)}</dd>
        </div>
        <div>
          <dt>Formality</dt>
          <dd>{formatNumber(analysis.style.formalityScore)}</dd>
        </div>
        <div>
          <dt>Mixed language</dt>
          <dd>{analysis.language.isMixed ? "Yes" : "No"}</dd>
        </div>
      </dl>
    </div>
  );
}

function TokensTable({ tokens }: { tokens: LinguisticToken[] }) {
  return (
    <Table>
      <thead>
        <tr>
          <th>#</th>
          <th>Kind</th>
          <th>Text</th>
          <th>Normalized</th>
          <th>Start</th>
          <th>End</th>
        </tr>
      </thead>
      <tbody>
        {tokens.map((token) => (
          <tr key={`${token.index}-${token.start}-${token.end}`}>
            <td>{token.index}</td>
            <td>{token.kind}</td>
            <td>{token.text}</td>
            <td>{token.normalized}</td>
            <td>{token.start}</td>
            <td>{token.end}</td>
          </tr>
        ))}
      </tbody>
    </Table>
  );
}

function SyntaxTables({
  lemmas,
  pos,
  sentences,
}: {
  lemmas: LinguisticLemma[];
  pos: LinguisticPos[];
  sentences: LinguisticSentence[];
}) {
  return (
    <div className="grid gap-4">
      <Table>
        <thead>
          <tr>
            <th>Sentence</th>
            <th>Text</th>
            <th>Tokens</th>
          </tr>
        </thead>
        <tbody>
          {sentences.map((sentence) => (
            <tr key={sentence.index}>
              <td>{sentence.index}</td>
              <td>{sentence.text}</td>
              <td>{sentence.tokenCount}</td>
            </tr>
          ))}
        </tbody>
      </Table>
      <Table>
        <thead>
          <tr>
            <th>#</th>
            <th>Token</th>
            <th>Lemma</th>
            <th>POS</th>
            <th>Reason</th>
          </tr>
        </thead>
        <tbody>
          {lemmas.map((lemma) => {
            const posTag = pos.find((item) => item.tokenIndex === lemma.tokenIndex);
            return (
              <tr key={lemma.tokenIndex}>
                <td>{lemma.tokenIndex}</td>
                <td>{lemma.token}</td>
                <td>{lemma.lemma}</td>
                <td>{posTag?.tag ?? "Unknown"}</td>
                <td>{posTag?.reason ?? ""}</td>
              </tr>
            );
          })}
        </tbody>
      </Table>
    </div>
  );
}

function EntitiesTable({ entities }: { entities: LinguisticEntity[] }) {
  return (
    <Table empty={entities.length === 0 ? "No named entities found." : undefined}>
      <thead>
        <tr>
          <th>ID</th>
          <th>Kind</th>
          <th>Text</th>
          <th>Normalized</th>
          <th>Sentence</th>
          <th>Confidence</th>
        </tr>
      </thead>
      <tbody>
        {entities.map((entity) => (
          <tr key={entity.id}>
            <td>{entity.id}</td>
            <td>{entity.kind}</td>
            <td>{entity.text}</td>
            <td>{entity.normalized}</td>
            <td>{entity.sentenceIndex}</td>
            <td>{formatNumber(entity.confidence)}</td>
          </tr>
        ))}
      </tbody>
    </Table>
  );
}

function EventsTable({
  events,
  relations,
}: {
  events: LinguisticEvent[];
  relations: LinguisticRelation[];
}) {
  return (
    <div className="grid gap-4">
      <Table empty={events.length === 0 ? "No events found." : undefined}>
        <thead>
          <tr>
            <th>Predicate</th>
            <th>Lemma</th>
            <th>Type</th>
            <th>Sentence</th>
            <th>Arguments</th>
          </tr>
        </thead>
        <tbody>
          {events.map((event, index) => (
            <tr key={`${event.sentenceIndex}-${event.predicate}-${index}`}>
              <td>{event.predicate}</td>
              <td>{event.lemma}</td>
              <td>{event.relationType}</td>
              <td>{event.sentenceIndex}</td>
              <td>
                {event.arguments.map((argument) => `${argument.role}: ${argument.text}`).join(", ")}
              </td>
            </tr>
          ))}
        </tbody>
      </Table>
      <Table empty={relations.length === 0 ? "No relations found." : undefined}>
        <thead>
          <tr>
            <th>Subject</th>
            <th>Relation</th>
            <th>Object</th>
            <th>Type</th>
            <th>Confidence</th>
          </tr>
        </thead>
        <tbody>
          {relations.map((relation, index) => (
            <tr key={`${relation.subject}-${relation.relation}-${relation.object}-${index}`}>
              <td>{relation.subject}</td>
              <td>{relation.relation}</td>
              <td>{relation.object}</td>
              <td>{relation.relationType}</td>
              <td>{formatNumber(relation.confidence)}</td>
            </tr>
          ))}
        </tbody>
      </Table>
    </div>
  );
}

function TopicsPanel({
  topics,
  analysis,
}: {
  topics: LinguisticTopic[];
  analysis: LinguisticAnalysisPayload;
}) {
  return (
    <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_280px]">
      <Table empty={topics.length === 0 ? "No topics found." : undefined}>
        <thead>
          <tr>
            <th>Label</th>
            <th>Terms</th>
            <th>Score</th>
          </tr>
        </thead>
        <tbody>
          {topics.map((topic) => (
            <tr key={topic.label}>
              <td>{topic.label}</td>
              <td>{topic.terms.join(", ")}</td>
              <td>{formatNumber(topic.score)}</td>
            </tr>
          ))}
        </tbody>
      </Table>
      <dl className={classNames(detailGridClass, "self-start sm:grid-cols-1")}>
        <div>
          <dt>Questions</dt>
          <dd>{analysis.style.questionCount}</dd>
        </div>
        <div>
          <dt>Exclamations</dt>
          <dd>{analysis.style.exclamationCount}</dd>
        </div>
        <div>
          <dt>Chunks</dt>
          <dd>{analysis.summary.chunkCount}</dd>
        </div>
        <div>
          <dt>Register</dt>
          <dd>{analysis.style.register}</dd>
        </div>
      </dl>
    </div>
  );
}

function JsonPanel({ json, onCopy }: { json: string; onCopy: () => void }) {
  return (
    <div>
      <div className="mb-3 flex justify-end">
        <button className={buttonSecondaryClass} type="button" onClick={onCopy}>
          Copy JSON
        </button>
      </div>
      <pre className="max-h-[30rem] overflow-auto rounded-md border border-zinc-200 bg-zinc-950 p-4 font-mono text-sm leading-6 text-zinc-50">
        {json}
      </pre>
    </div>
  );
}

function Table({ children, empty }: { children: ReactNode; empty?: string }) {
  if (empty) {
    return (
      <div className="rounded-md border border-dashed border-zinc-300 bg-zinc-50 p-5 text-sm text-zinc-500">
        {empty}
      </div>
    );
  }
  return (
    <div className={tableWrapClass}>
      <table className={dataTableClass}>{children}</table>
    </div>
  );
}

function describeCurrentModel({
  analysis,
  fallbackModelId,
  model,
  nlpTask,
  runtimeMode,
  taskResult,
}: {
  analysis: LinguisticAnalysisPayload | null;
  fallbackModelId: string;
  model?: NlpModelMetadata;
  nlpTask: NlpTask;
  runtimeMode: RuntimeMode;
  taskResult: unknown | null;
}): { label: string; detail: string } {
  if (nlpTask === "entities" && analysis?.model) {
    const entityModel = analysis.model.entityModel ?? analysis.model.entityRecognition;
    return {
      label: entityModel,
      detail: `${formatRuntime(analysis.model.entityRecognition)} entity recognition, ${formatRuntime(
        analysis.model.tokenizerMode,
      )} tokenizer`,
    };
  }

  const taskModel = taskResultModel(taskResult, nlpTask);
  if (taskModel) {
    return taskModel;
  }

  if (nlpTask === "entities" && runtimeMode === "client-wasm") {
    return {
      label: "heuristic",
      detail: "Client WASM rule-based entity recognition",
    };
  }

  return {
    label: model?.id ?? fallbackModelId,
    detail: model
      ? `${formatRuntime(model.runtime)} runtime, ${model.supported ? "available" : "fallback-gated"}`
      : "Default preset",
  };
}

function taskResultModel(
  value: unknown,
  nlpTask: NlpTask,
): { label: string; detail: string } | null {
  if (!value || typeof value !== "object") {
    return null;
  }
  const result = value as { modelId?: unknown; operation?: unknown; runtime?: unknown };
  if (typeof result.modelId !== "string" || !taskMatchesResult(nlpTask, result.operation)) {
    return null;
  }
  const runtime = typeof result.runtime === "string" ? result.runtime : "unknown";
  return {
    label: result.modelId,
    detail: `${formatRuntime(runtime)} runtime from the last result`,
  };
}

function taskMatchesResult(nlpTask: NlpTask, operation: unknown): boolean {
  if (typeof operation !== "string") {
    return false;
  }
  const operationByTask: Record<NlpTask, string> = {
    entities: "analyze",
    sentiment: "sentiment",
    classify: "classify",
    embed: "embed",
    "zero-shot": "zero-shot",
    summarize: "summarize",
    rerank: "rerank",
    qa: "question-answer",
  };
  return operationByTask[nlpTask] === operation;
}

function formatRuntime(value: string): string {
  return value.replace(/[_-]+/g, " ").replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function formatNumber(value: number): string {
  if (Number.isInteger(value)) {
    return value.toLocaleString();
  }
  return value.toLocaleString(undefined, { maximumFractionDigits: 3 });
}

function classNames(...values: Array<string | false | null | undefined>): string {
  return values.filter(Boolean).join(" ");
}
