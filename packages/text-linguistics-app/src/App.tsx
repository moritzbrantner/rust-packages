import { FormEvent, useMemo, useState, type ReactNode } from "react";

import {
  analyzeLinguistics,
  analyzeLinguisticsClient,
  serverBaseUrl,
  type LinguisticAnalysisPayload,
  type LinguisticEntity,
  type LinguisticEvent,
  type LinguisticLemma,
  type LinguisticPos,
  type LinguisticRelation,
  type LinguisticSentence,
  type LinguisticToken,
  type LinguisticTopic,
} from "./api";
import { sampleText } from "./sampleText";

type LoadState = "idle" | "loading" | "ready" | "error";
type RuntimeMode = "server" | "client-wasm";
type ActiveTab = "overview" | "tokens" | "syntax" | "entities" | "events" | "topics" | "json";

const tabs: Array<{ id: ActiveTab; label: string }> = [
  { id: "overview", label: "Overview" },
  { id: "tokens", label: "Tokens" },
  { id: "syntax", label: "Syntax" },
  { id: "entities", label: "Entities" },
  { id: "events", label: "Events" },
  { id: "topics", label: "Topics" },
  { id: "json", label: "JSON" },
];

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
  const [loadState, setLoadState] = useState<LoadState>("idle");
  const [runtimeMode, setRuntimeMode] = useState<RuntimeMode>("server");
  const [activeTab, setActiveTab] = useState<ActiveTab>("overview");
  const [error, setError] = useState<string | null>(null);

  const json = useMemo(() => (analysis ? JSON.stringify(analysis, null, 2) : ""), [analysis]);
  const statusLabel =
    loadState === "ready" ? "Ready" : loadState === "loading" ? "Analyzing" : loadState === "error" ? "Error" : "Idle";

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
      const payload =
        runtimeMode === "server" ? await analyzeLinguistics(text) : await analyzeLinguisticsClient(text);
      setAnalysis(payload);
      setLoadState("ready");
      setActiveTab("overview");
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
            <button className={buttonSecondaryClass} type="button" onClick={() => setText(sampleText)}>
              Reset sample
            </button>
            <button className={buttonSecondaryClass} type="button" onClick={() => setText("")}>
              Clear
            </button>
            <button className={buttonSecondaryClass} type="button" disabled={!json} onClick={copyJson}>
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
                {runtimeMode === "server" ? serverBaseUrl : "Client WASM"}
              </p>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <RuntimeButton active={runtimeMode === "server"} onClick={() => setRuntimeMode("server")}>
                Server
              </RuntimeButton>
              <RuntimeButton active={runtimeMode === "client-wasm"} onClick={() => setRuntimeMode("client-wasm")}>
                Client WASM
              </RuntimeButton>
              <button className={buttonPrimaryClass} type="submit" disabled={loadState === "loading"}>
                Analyze
              </button>
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
          <div className="flex flex-col gap-4">
            <div className="flex flex-wrap gap-2">
              {tabs.map((tab) => (
                <button
                  key={tab.id}
                  className={classNames(tabButtonClass, activeTab === tab.id ? tabActiveClass : "")}
                  type="button"
                  onClick={() => setActiveTab(tab.id)}
                >
                  {tab.label}
                </button>
              ))}
            </div>
            {analysis ? (
              <AnalysisPanel
                activeTab={activeTab}
                analysis={analysis}
                json={json}
                onCopyJson={copyJson}
              />
            ) : (
              <div className="flex min-h-80 items-center justify-center rounded-md border border-dashed border-zinc-300 bg-zinc-50 text-sm font-medium text-zinc-500">
                Run analysis to populate linguistic annotations.
              </div>
            )}
          </div>
        </section>
      </section>
    </main>
  );
}

function AnalysisPanel({
  activeTab,
  analysis,
  json,
  onCopyJson,
}: {
  activeTab: ActiveTab;
  analysis: LinguisticAnalysisPayload;
  json: string;
  onCopyJson: () => void;
}) {
  if (activeTab === "overview") {
    return <Overview analysis={analysis} />;
  }
  if (activeTab === "tokens") {
    return <TokensTable tokens={analysis.tokens} />;
  }
  if (activeTab === "syntax") {
    return <SyntaxTables lemmas={analysis.lemmas} pos={analysis.pos} sentences={analysis.sentences} />;
  }
  if (activeTab === "entities") {
    return <EntitiesTable entities={analysis.entities} />;
  }
  if (activeTab === "events") {
    return <EventsTable events={analysis.events} relations={analysis.relations} />;
  }
  if (activeTab === "topics") {
    return <TopicsPanel topics={analysis.topics} analysis={analysis} />;
  }
  return <JsonPanel json={json} onCopy={onCopyJson} />;
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
              <td>{event.arguments.map((argument) => `${argument.role}: ${argument.text}`).join(", ")}</td>
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

function formatNumber(value: number): string {
  if (Number.isInteger(value)) {
    return value.toLocaleString();
  }
  return value.toLocaleString(undefined, { maximumFractionDigits: 3 });
}

function classNames(...values: Array<string | false | null | undefined>): string {
  return values.filter(Boolean).join(" ");
}
