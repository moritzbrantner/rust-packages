import { useEffect, useMemo, useState } from "react";
import { initializeWasm, runOperation, type RuntimeMode } from "./api";

const sampleText =
  "Alice presented the tokenizer roadmap in Berlin. Rust crates analyze text with deterministic local features. Semantic search and lexical statistics support transcript workflows.";

type Tab = "overview" | "stats" | "lexical" | "similarity" | "linguistics" | "embedding" | "json";

export function App() {
  const [mode, setMode] = useState<RuntimeMode>("client-wasm");
  const [profile, setProfile] = useState("deterministic");
  const [text, setText] = useState(sampleText);
  const [keywordLimit, setKeywordLimit] = useState(10);
  const [summarySentences, setSummarySentences] = useState(3);
  const [embeddingDimensions, setEmbeddingDimensions] = useState(128);
  const [linguistics, setLinguistics] = useState(true);
  const [embeddings, setEmbeddings] = useState(true);
  const [shingles, setShingles] = useState(true);
  const [activeTab, setActiveTab] = useState<Tab>("overview");
  const [result, setResult] = useState<any>(null);
  const [status, setStatus] = useState("idle");

  useEffect(() => {
    initializeWasm().catch(() => undefined);
  }, []);

  const request = useMemo(
    () => ({
      id: "app-input",
      text,
      profile,
      keywordLimit,
      summarySentences,
      ngramSizes: shingles ? [2, 3] : [],
      shingleSizes: shingles ? [3, 5] : [3],
      linguistics: { mode: linguistics ? "heuristicBalanced" : "off" },
      embedding: embeddings
        ? { mode: "hashed", dimensions: embeddingDimensions, useIdf: false }
        : { mode: "off" },
    }),
    [embeddingDimensions, embeddings, keywordLimit, linguistics, profile, shingles, summarySentences, text],
  );

  useEffect(() => {
    if (profile === "modelBacked" && mode !== "server") {
      setMode("server");
    }
  }, [mode, profile]);

  async function run() {
    setStatus("loading");
    try {
      const response = await runOperation(mode, "analysis.document", request);
      setResult(response.value);
      setStatus(response.diagnostics.length ? "diagnostic" : "ready");
    } catch (error) {
      setResult({ error: error instanceof Error ? error.message : String(error) });
      setStatus("error");
    }
  }

  const tabs: Tab[] = ["overview", "stats", "lexical", "similarity", "linguistics", "embedding", "json"];

  return (
    <main className="min-h-screen">
      <header className="border-b border-slate-200 bg-white px-5 py-4">
        <div className="mx-auto flex max-w-screen-2xl flex-wrap items-center justify-between gap-3">
          <div>
            <h1 className="text-lg font-semibold">text-analysis</h1>
            <div className="mt-1 text-sm text-slate-500">{status}</div>
          </div>
          <div className="flex flex-wrap gap-2">
            <select className="h-10 rounded border border-slate-300 bg-white px-3" value={mode} onChange={(event) => setMode(event.target.value as RuntimeMode)}>
              <option value="client-wasm" disabled={profile === "modelBacked"}>Client WASM</option>
              <option value="server">Server</option>
            </select>
            <button className="h-10 rounded bg-slate-950 px-4 font-semibold text-white disabled:opacity-50" disabled={status === "loading"} onClick={run}>
              Run
            </button>
          </div>
        </div>
      </header>
      <section className="mx-auto grid max-w-screen-2xl gap-5 px-5 py-5 xl:grid-cols-[minmax(360px,0.8fr)_minmax(0,1.2fr)]">
        <form className="rounded border border-slate-200 bg-white p-4" onSubmit={(event) => { event.preventDefault(); void run(); }}>
          <div className="grid gap-3 sm:grid-cols-2">
            <label className="grid gap-1 text-sm font-medium">
              Profile
              <select className="h-10 rounded border border-slate-300 px-3" value={profile} onChange={(event) => setProfile(event.target.value)}>
                <option value="deterministic">Deterministic</option>
                <option value="modelBacked">Model-backed</option>
              </select>
            </label>
            <label className="grid gap-1 text-sm font-medium">
              Embedding dimensions
              <input className="h-10 rounded border border-slate-300 px-3" min={8} type="number" value={embeddingDimensions} onChange={(event) => setEmbeddingDimensions(Number(event.target.value))} />
            </label>
            <label className="grid gap-1 text-sm font-medium">
              Keywords
              <input className="h-10 rounded border border-slate-300 px-3" min={1} type="number" value={keywordLimit} onChange={(event) => setKeywordLimit(Number(event.target.value))} />
            </label>
            <label className="grid gap-1 text-sm font-medium">
              Summary sentences
              <input className="h-10 rounded border border-slate-300 px-3" min={1} type="number" value={summarySentences} onChange={(event) => setSummarySentences(Number(event.target.value))} />
            </label>
          </div>
          <div className="mt-4 flex flex-wrap gap-4 text-sm font-medium">
            <label className="flex items-center gap-2"><input type="checkbox" checked={linguistics} onChange={(event) => setLinguistics(event.target.checked)} /> Linguistics</label>
            <label className="flex items-center gap-2"><input type="checkbox" checked={embeddings} onChange={(event) => setEmbeddings(event.target.checked)} /> Embeddings</label>
            <label className="flex items-center gap-2"><input type="checkbox" checked={shingles} onChange={(event) => setShingles(event.target.checked)} /> Shingles</label>
          </div>
          <textarea className="mt-4 min-h-80 w-full resize-y rounded border border-slate-300 bg-slate-950 p-4 font-mono text-sm leading-6 text-white" value={text} onChange={(event) => setText(event.target.value)} />
        </form>
        <section className="rounded border border-slate-200 bg-white p-4">
          <div className="flex flex-wrap gap-2 border-b border-slate-200 pb-3">
            {tabs.map((tab) => (
              <button key={tab} className={`rounded px-3 py-2 text-sm font-semibold ${activeTab === tab ? "bg-slate-950 text-white" : "bg-slate-100 text-slate-700"}`} onClick={() => setActiveTab(tab)}>
                {tab}
              </button>
            ))}
          </div>
          <pre className="mt-4 max-h-[42rem] overflow-auto rounded bg-slate-950 p-4 text-sm leading-6 text-slate-50">
            {JSON.stringify(selectTab(result, activeTab), null, 2)}
          </pre>
        </section>
      </section>
    </main>
  );
}

function selectTab(result: any, tab: Tab) {
  if (!result) {
    return {};
  }
  if (tab === "overview") {
    return {
      id: result.id,
      language: result.language,
      diagnostics: result.diagnostics,
      words: result.core?.stats?.basic?.words,
      keywords: result.lexical?.keywords?.slice?.(0, 5),
    };
  }
  if (tab === "stats") return { core: result.core, enrichedStats: result.enrichedStats };
  if (tab === "json") return result;
  return result[tab] ?? {};
}
