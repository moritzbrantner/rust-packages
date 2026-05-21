import { useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from "react";

import {
  countParagraphSentences,
  countSentenceTokens,
  filterTokens,
  formatNumber,
  tokenKindClass,
  tokenKindFilters,
  tokenKindLabel,
  type TextSpan,
  type TextToken,
  type TokenKindFilter,
} from "./analysis";
import { sampleText } from "./sampleText";
import {
  analyzeText,
  initTextCoreWasm,
  type TextDocumentAnalysis,
  type TextProcessingOptions,
} from "./textCoreWasm";

type WasmState = "loading" | "ready" | "unavailable";
type ActiveTab = "overview" | "tokens" | "sentences" | "paragraphs" | "scripts" | "json";
type SelectedSpan =
  | { kind: "token"; index: number; span: TextToken }
  | { kind: "sentence"; index: number; span: TextSpan }
  | { kind: "paragraph"; index: number; span: TextSpan };

const tabs: Array<{ id: ActiveTab; label: string }> = [
  { id: "overview", label: "Overview" },
  { id: "tokens", label: "Tokens" },
  { id: "sentences", label: "Sentences" },
  { id: "paragraphs", label: "Paragraphs" },
  { id: "scripts", label: "Scripts" },
  { id: "json", label: "JSON" },
];

const defaultOptions: Required<TextProcessingOptions> = {
  lowercase: true,
  normalizeUnicode: true,
  keepApostrophes: true,
  includePunctuation: false,
  includeTokens: true,
};

const processingOptions: Array<{
  key: keyof Required<TextProcessingOptions>;
  label: string;
  onLabel: string;
  offLabel: string;
}> = [
  {
    key: "lowercase",
    label: "Token case",
    onLabel: "Normalize to lowercase",
    offLabel: "Preserve original case",
  },
  {
    key: "normalizeUnicode",
    label: "Unicode form",
    onLabel: "Normalize compatible characters",
    offLabel: "Preserve original codepoints",
  },
  {
    key: "keepApostrophes",
    label: "Apostrophes",
    onLabel: "Keep contractions together",
    offLabel: "Split at apostrophes",
  },
  {
    key: "includePunctuation",
    label: "Punctuation tokens",
    onLabel: "Include punctuation",
    offLabel: "Ignore punctuation",
  },
  {
    key: "includeTokens",
    label: "Token output",
    onLabel: "Show token spans",
    offLabel: "Only segment text",
  },
];

const panelClass = "min-w-0 rounded-md border border-zinc-200 bg-white p-5 shadow-sm";
const buttonPrimaryClass =
  "rounded-md bg-sky-700 px-3 py-2 text-sm font-semibold text-white transition hover:bg-sky-800 focus:outline-none focus:ring-2 focus:ring-sky-600 focus:ring-offset-2 disabled:cursor-not-allowed disabled:bg-zinc-300";
const buttonSecondaryClass =
  "rounded-md border border-zinc-300 bg-white px-3 py-2 text-sm font-medium text-zinc-800 transition hover:bg-zinc-100 focus:outline-none focus:ring-2 focus:ring-sky-600 focus:ring-offset-2";
const statusPillClass =
  "inline-flex min-h-9 min-w-28 items-center justify-center rounded-md px-3 text-sm font-semibold";
const sectionTitleClass = "text-base font-semibold text-zinc-950";
const textEditorClass =
  "min-h-44 w-full resize-none overflow-hidden rounded-md border border-zinc-300 bg-zinc-950 p-4 font-mono text-sm leading-6 text-zinc-50 outline-none focus:border-sky-500 focus:ring-2 focus:ring-sky-200";
const toggleRowClass =
  "flex min-h-16 items-center gap-3 rounded-md border border-zinc-200 bg-zinc-50 px-3 py-2 text-sm font-medium text-zinc-800";
const toggleInputClass = "h-4 w-4 shrink-0 rounded border-zinc-300 text-sky-700 focus:ring-sky-600";
const toggleStateClass = "mt-0.5 block text-xs font-normal leading-5 text-zinc-500";
const tabButtonClass =
  "min-h-9 rounded-md border border-zinc-200 bg-white px-3 text-sm font-medium text-zinc-700 transition hover:border-zinc-300 hover:bg-zinc-50";
const tabActiveClass =
  "border-zinc-950 bg-zinc-950 text-white hover:border-zinc-950 hover:bg-zinc-950";
const textViewerClass =
  "min-h-44 max-h-72 overflow-auto whitespace-pre-wrap rounded-md border border-zinc-200 bg-zinc-50 p-4 font-mono text-sm leading-7 text-zinc-900";
const inlineHighlightClass =
  "mx-0.5 rounded px-1 py-0.5 font-mono outline-none ring-offset-1 transition focus:ring-2";
const segmentSentenceClass = "bg-sky-100 text-sky-950 focus:ring-sky-500";
const segmentParagraphClass = "bg-emerald-100 text-emerald-950 focus:ring-emerald-500";
const statsGridClass = "grid gap-3 sm:grid-cols-2 lg:grid-cols-4";
const statCellClass = "rounded-md border border-zinc-200 bg-zinc-50 p-3";
const statLabelClass = "text-xs font-semibold text-zinc-500";
const statValueClass = "mt-1 text-xl font-semibold text-zinc-950";
const detailGridClass =
  "grid gap-3 text-sm sm:grid-cols-2 [&_dd]:mt-1 [&_dd]:break-words [&_dd]:font-mono [&_dd]:text-zinc-900 [&_div]:min-w-0 [&_div]:rounded-md [&_div]:border [&_div]:border-zinc-200 [&_div]:bg-zinc-50 [&_div]:p-3 [&_dt]:text-xs [&_dt]:font-semibold [&_dt]:text-zinc-500";
const filterRowClass =
  "grid gap-3 sm:grid-cols-[180px_minmax(0,1fr)] [&_input]:min-h-10 [&_input]:rounded-md [&_input]:border [&_input]:border-zinc-300 [&_input]:bg-white [&_input]:px-3 [&_input]:text-sm [&_input]:text-zinc-950 [&_input]:outline-none [&_input:focus]:border-sky-500 [&_input:focus]:ring-2 [&_input:focus]:ring-sky-200 [&_label]:grid [&_label]:gap-1 [&_label]:text-sm [&_label]:font-medium [&_label]:text-zinc-700 [&_select]:min-h-10 [&_select]:rounded-md [&_select]:border [&_select]:border-zinc-300 [&_select]:bg-white [&_select]:px-3 [&_select]:text-sm [&_select]:text-zinc-950 [&_select]:outline-none [&_select:focus]:border-sky-500 [&_select:focus]:ring-2 [&_select:focus]:ring-sky-200";
const tableWrapClass = "max-h-80 overflow-auto rounded-md border border-zinc-200";
const dataTableClass =
  "w-full border-collapse text-left text-sm [&_tbody_tr]:cursor-pointer [&_tbody_tr:hover]:bg-zinc-50 [&_td]:max-w-xs [&_td]:border-t [&_td]:border-zinc-200 [&_td]:px-3 [&_td]:py-2 [&_td]:align-top [&_td]:text-zinc-900 [&_th]:sticky [&_th]:top-0 [&_th]:bg-zinc-100 [&_th]:px-3 [&_th]:py-2 [&_th]:font-semibold [&_th]:text-zinc-700";
const kindBadgeClass = "inline-flex rounded px-2 py-1 text-xs font-semibold";
const jsonBlockClass =
  "max-h-[30rem] overflow-auto rounded-md border border-zinc-200 bg-zinc-950 p-4 font-mono text-sm leading-6 text-zinc-50";
const selectedPanelClass = "mt-4 border-t border-zinc-200 pt-4";
const selectedTextClass =
  "mt-3 max-h-32 overflow-auto whitespace-pre-wrap rounded-md bg-zinc-100 p-3 font-mono text-sm text-zinc-900";
const errorTextClass =
  "rounded-md border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-800";
const emptyStateClass =
  "flex min-h-80 items-center justify-center rounded-md border border-dashed border-zinc-300 bg-zinc-50 text-sm font-medium text-zinc-500";

export function App() {
  const [wasmState, setWasmState] = useState<WasmState>("loading");
  const [error, setError] = useState<string | null>(null);
  const [text, setText] = useState(sampleText);
  const [options, setOptions] = useState<Required<TextProcessingOptions>>(defaultOptions);
  const [activeTab, setActiveTab] = useState<ActiveTab>("overview");
  const [selectedSpan, setSelectedSpan] = useState<SelectedSpan | null>(null);
  const [tokenKindFilter, setTokenKindFilter] = useState<TokenKindFilter>("all");
  const [tokenQuery, setTokenQuery] = useState("");
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);

  useEffect(() => {
    let cancelled = false;
    initTextCoreWasm()
      .then(() => {
        if (!cancelled) {
          setWasmState("ready");
        }
      })
      .catch((caught) => {
        if (!cancelled) {
          setWasmState("unavailable");
          setError(caught instanceof Error ? caught.message : String(caught));
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useLayoutEffect(() => {
    const textarea = textareaRef.current;
    if (!textarea) {
      return;
    }
    textarea.style.height = "auto";
    textarea.style.height = `${textarea.scrollHeight}px`;
  }, [text]);

  const analysisResult = useMemo<{
    analysis: TextDocumentAnalysis | null;
    error: string | null;
  }>(() => {
    if (wasmState !== "ready") {
      return { analysis: null, error: null };
    }
    try {
      return { analysis: analyzeText(text, options), error: null };
    } catch (caught) {
      return {
        analysis: null,
        error: caught instanceof Error ? caught.message : String(caught),
      };
    }
  }, [options, text, wasmState]);
  const analysis = analysisResult.analysis;
  const visibleError = error ?? analysisResult.error;

  const filteredTokens = useMemo(
    () => filterTokens(analysis?.tokens ?? [], tokenKindFilter, tokenQuery),
    [analysis?.tokens, tokenKindFilter, tokenQuery],
  );
  const json = useMemo(() => (analysis ? JSON.stringify(analysis, null, 2) : ""), [analysis]);
  const statusLabel =
    wasmState === "ready" ? "WASM ready" : wasmState === "loading" ? "Loading" : "Unavailable";

  function updateOption<K extends keyof Required<TextProcessingOptions>>(
    key: K,
    value: Required<TextProcessingOptions>[K],
  ) {
    setOptions((current) => ({ ...current, [key]: value }));
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
            <div className="text-sm font-semibold text-sky-700">Package app</div>
            <h1 className="mt-1 text-2xl font-semibold">Text Core</h1>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <span
              className={classNames(
                statusPillClass,
                wasmState === "ready"
                  ? "bg-emerald-100 text-emerald-800"
                  : wasmState === "loading"
                    ? "bg-amber-100 text-amber-800"
                    : "bg-rose-100 text-rose-800",
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
              className={buttonPrimaryClass}
              type="button"
              disabled={!json}
              onClick={copyJson}
            >
              Copy JSON
            </button>
          </div>
        </div>
      </header>

      <section className="mx-auto grid w-full max-w-screen-2xl gap-5 px-5 py-5">
        <section className={panelClass}>
          <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
            <h2 className={sectionTitleClass}>Input</h2>
            <div className="text-sm text-zinc-500">{formatNumber(text.length)} UTF-16 units</div>
          </div>
          <textarea
            ref={textareaRef}
            className={classNames(textEditorClass, "mt-4")}
            spellCheck={false}
            value={text}
            onChange={(event) => setText(event.target.value)}
          />
          <div className="mt-4">
            <h2 className={sectionTitleClass}>Processing</h2>
            <div className="mt-3 grid gap-3 md:grid-cols-2 xl:grid-cols-3">
              {processingOptions.map((option) => (
                <Toggle
                  checked={options[option.key]}
                  key={option.key}
                  label={option.label}
                  stateLabel={options[option.key] ? option.onLabel : option.offLabel}
                  onChange={(checked) => updateOption(option.key, checked)}
                />
              ))}
            </div>
          </div>
          {selectedSpan ? <SelectedSpanPanel selected={selectedSpan} /> : null}
        </section>

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
            {visibleError ? <p className={errorTextClass}>{visibleError}</p> : null}
            {analysis ? (
              <>
                <TextViewer
                  activeTab={activeTab}
                  analysis={analysis}
                  filteredTokens={filteredTokens}
                  text={text}
                  onSelect={setSelectedSpan}
                />
                {activeTab === "overview" ? <Overview analysis={analysis} /> : null}
                {activeTab === "tokens" ? (
                  <TokensPanel
                    filteredTokens={filteredTokens}
                    kind={tokenKindFilter}
                    query={tokenQuery}
                    tokens={analysis.tokens}
                    onKindChange={setTokenKindFilter}
                    onQueryChange={setTokenQuery}
                    onSelect={(index, span) => setSelectedSpan({ kind: "token", index, span })}
                  />
                ) : null}
                {activeTab === "sentences" ? (
                  <SentencesPanel
                    analysis={analysis}
                    onSelect={(index, span) => setSelectedSpan({ kind: "sentence", index, span })}
                  />
                ) : null}
                {activeTab === "paragraphs" ? (
                  <ParagraphsPanel
                    analysis={analysis}
                    onSelect={(index, span) => setSelectedSpan({ kind: "paragraph", index, span })}
                  />
                ) : null}
                {activeTab === "scripts" ? <ScriptsPanel analysis={analysis} /> : null}
                {activeTab === "json" ? <JsonPanel json={json} onCopy={copyJson} /> : null}
              </>
            ) : (
              <div className={emptyStateClass}>{statusLabel}</div>
            )}
          </div>
        </section>
      </section>
    </main>
  );
}

function Toggle({
  checked,
  label,
  stateLabel,
  onChange,
}: {
  checked: boolean;
  label: string;
  stateLabel: string;
  onChange: (checked: boolean) => void;
}) {
  return (
    <label className={toggleRowClass}>
      <input
        className={toggleInputClass}
        checked={checked}
        type="checkbox"
        onChange={(event) => onChange(event.target.checked)}
      />
      <span className="min-w-0">
        <span className="block">{label}</span>
        <span className={toggleStateClass}>{stateLabel}</span>
      </span>
    </label>
  );
}

function TextViewer({
  activeTab,
  analysis,
  filteredTokens,
  text,
  onSelect,
}: {
  activeTab: ActiveTab;
  analysis: TextDocumentAnalysis;
  filteredTokens: TextToken[];
  text: string;
  onSelect: (selected: SelectedSpan) => void;
}) {
  const highlights = useMemo(() => {
    if (activeTab === "tokens") {
      const tokenIndex = new Map(analysis.tokens.map((token, index) => [token, index]));
      return filteredTokens.map((token) => ({
        className: tokenKindClass(token.kind),
        index: tokenIndex.get(token) ?? -1,
        kind: "token" as const,
        span: token,
      }));
    }
    if (activeTab === "sentences") {
      return analysis.sentences.map((sentence, index) => ({
        className: segmentSentenceClass,
        index,
        kind: "sentence" as const,
        span: sentence,
      }));
    }
    if (activeTab === "paragraphs") {
      return analysis.paragraphs.map((paragraph, index) => ({
        className: segmentParagraphClass,
        index,
        kind: "paragraph" as const,
        span: paragraph,
      }));
    }
    return analysis.tokens.map((token, index) => ({
      className: tokenKindClass(token.kind),
      index,
      kind: "token" as const,
      span: token,
    }));
  }, [activeTab, analysis.paragraphs, analysis.sentences, analysis.tokens, filteredTokens]);

  return <div className={textViewerClass}>{renderHighlightedText(text, highlights, onSelect)}</div>;
}

function renderHighlightedText(
  text: string,
  highlights: Array<{
    className: string;
    index: number;
    kind: SelectedSpan["kind"];
    span: TextSpan | TextToken;
  }>,
  onSelect: (selected: SelectedSpan) => void,
) {
  const parts: ReactNode[] = [];
  let cursor = 0;
  for (const highlight of highlights.sort((left, right) => left.span.start - right.span.start)) {
    if (highlight.span.start < cursor) {
      continue;
    }
    if (highlight.span.start > cursor) {
      parts.push(text.slice(cursor, highlight.span.start));
    }
    const segmentText = text.slice(highlight.span.start, highlight.span.end);
    parts.push(
      <button
        key={`${highlight.kind}-${highlight.index}-${highlight.span.start}`}
        className={classNames(inlineHighlightClass, highlight.className)}
        type="button"
        onClick={() => {
          if (highlight.kind === "token") {
            onSelect({ kind: "token", index: highlight.index, span: highlight.span as TextToken });
          } else if (highlight.kind === "sentence") {
            onSelect({
              kind: "sentence",
              index: highlight.index,
              span: highlight.span as TextSpan,
            });
          } else {
            onSelect({
              kind: "paragraph",
              index: highlight.index,
              span: highlight.span as TextSpan,
            });
          }
        }}
      >
        {segmentText}
      </button>,
    );
    cursor = highlight.span.end;
  }
  if (cursor < text.length) {
    parts.push(text.slice(cursor));
  }
  return parts.length > 0 ? parts : text;
}

function Overview({ analysis }: { analysis: TextDocumentAnalysis }) {
  const stats = [
    ["Bytes", analysis.stats.bytes],
    ["Chars", analysis.stats.chars],
    ["Words", analysis.stats.words],
    ["Lines", analysis.stats.lines],
    ["Sentences", analysis.stats.sentences],
    ["Paragraphs", analysis.stats.paragraphs],
    ["Tokens", analysis.stats.tokens],
    ["Unique tokens", analysis.stats.uniqueTokens],
  ] as const;

  return (
    <div className="space-y-4">
      <div className={statsGridClass}>
        {stats.map(([label, value]) => (
          <div className={statCellClass} key={label}>
            <div className={statLabelClass}>{label}</div>
            <div className={statValueClass}>{formatNumber(value)}</div>
          </div>
        ))}
      </div>
      <dl className={detailGridClass}>
        <div>
          <dt>Words per sentence</dt>
          <dd>{formatNumber(analysis.stats.averageWordsPerSentence)}</dd>
        </div>
        <div>
          <dt>Chars per word</dt>
          <dd>{formatNumber(analysis.stats.averageCharsPerWord)}</dd>
        </div>
        <div>
          <dt>Dominant script</dt>
          <dd>{analysis.scriptProfile.dominantScript ?? "None"}</dd>
        </div>
        <div>
          <dt>Mixed script</dt>
          <dd>{analysis.scriptProfile.isMixed ? "Yes" : "No"}</dd>
        </div>
      </dl>
    </div>
  );
}

function TokensPanel({
  filteredTokens,
  kind,
  query,
  tokens,
  onKindChange,
  onQueryChange,
  onSelect,
}: {
  filteredTokens: TextToken[];
  kind: TokenKindFilter;
  query: string;
  tokens: TextToken[];
  onKindChange: (kind: TokenKindFilter) => void;
  onQueryChange: (query: string) => void;
  onSelect: (index: number, span: TextToken) => void;
}) {
  return (
    <div className="space-y-4">
      <div className={filterRowClass}>
        <label>
          <span>Kind</span>
          <select
            value={kind}
            onChange={(event) => onKindChange(event.target.value as TokenKindFilter)}
          >
            {tokenKindFilters.map((filter) => (
              <option key={filter} value={filter}>
                {tokenKindLabel(filter)}
              </option>
            ))}
          </select>
        </label>
        <label>
          <span>Filter</span>
          <input
            value={query}
            type="search"
            onChange={(event) => onQueryChange(event.target.value)}
          />
        </label>
      </div>
      <DataTable>
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
          {filteredTokens.map((token) => {
            const index = tokens.indexOf(token);
            return (
              <tr
                key={`${token.start}-${token.end}-${index}`}
                onClick={() => onSelect(index, token)}
              >
                <td>{index}</td>
                <td>
                  <span className={classNames(kindBadgeClass, tokenKindClass(token.kind))}>
                    {tokenKindLabel(token.kind)}
                  </span>
                </td>
                <td>{token.text}</td>
                <td>{token.normalized}</td>
                <td>{token.start}</td>
                <td>{token.end}</td>
              </tr>
            );
          })}
        </tbody>
      </DataTable>
    </div>
  );
}

function SentencesPanel({
  analysis,
  onSelect,
}: {
  analysis: TextDocumentAnalysis;
  onSelect: (index: number, span: TextSpan) => void;
}) {
  return (
    <DataTable>
      <thead>
        <tr>
          <th>#</th>
          <th>Text</th>
          <th>Start</th>
          <th>End</th>
          <th>Tokens</th>
        </tr>
      </thead>
      <tbody>
        {analysis.sentences.map((sentence, index) => (
          <tr key={`${sentence.start}-${sentence.end}`} onClick={() => onSelect(index, sentence)}>
            <td>{index}</td>
            <td>{sentence.text}</td>
            <td>{sentence.start}</td>
            <td>{sentence.end}</td>
            <td>{countSentenceTokens(sentence, analysis.tokens)}</td>
          </tr>
        ))}
      </tbody>
    </DataTable>
  );
}

function ParagraphsPanel({
  analysis,
  onSelect,
}: {
  analysis: TextDocumentAnalysis;
  onSelect: (index: number, span: TextSpan) => void;
}) {
  return (
    <DataTable>
      <thead>
        <tr>
          <th>#</th>
          <th>Text</th>
          <th>Start</th>
          <th>End</th>
          <th>Sentences</th>
        </tr>
      </thead>
      <tbody>
        {analysis.paragraphs.map((paragraph, index) => (
          <tr
            key={`${paragraph.start}-${paragraph.end}`}
            onClick={() => onSelect(index, paragraph)}
          >
            <td>{index}</td>
            <td>{paragraph.text}</td>
            <td>{paragraph.start}</td>
            <td>{paragraph.end}</td>
            <td>{countParagraphSentences(paragraph, analysis.sentences)}</td>
          </tr>
        ))}
      </tbody>
    </DataTable>
  );
}

function ScriptsPanel({ analysis }: { analysis: TextDocumentAnalysis }) {
  const scripts = Object.entries(analysis.scriptProfile.scripts).sort(
    (left, right) => right[1] - left[1] || left[0].localeCompare(right[0]),
  );
  return (
    <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_260px]">
      <DataTable>
        <thead>
          <tr>
            <th>Script</th>
            <th>Count</th>
          </tr>
        </thead>
        <tbody>
          {scripts.map(([script, count]) => (
            <tr key={script}>
              <td>{script}</td>
              <td>{count}</td>
            </tr>
          ))}
        </tbody>
      </DataTable>
      <dl className={classNames(detailGridClass, "self-start")}>
        <div>
          <dt>Digits</dt>
          <dd>{analysis.scriptProfile.digits}</dd>
        </div>
        <div>
          <dt>Whitespace</dt>
          <dd>{analysis.scriptProfile.whitespace}</dd>
        </div>
        <div>
          <dt>Punctuation</dt>
          <dd>{analysis.scriptProfile.punctuation}</dd>
        </div>
        <div>
          <dt>Other</dt>
          <dd>{analysis.scriptProfile.other}</dd>
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
      <pre className={jsonBlockClass}>{json}</pre>
    </div>
  );
}

function SelectedSpanPanel({ selected }: { selected: SelectedSpan }) {
  return (
    <section className={selectedPanelClass}>
      <h2 className={sectionTitleClass}>Selection</h2>
      <dl className={classNames(detailGridClass, "mt-3")}>
        <div>
          <dt>Type</dt>
          <dd>{selected.kind}</dd>
        </div>
        <div>
          <dt>Index</dt>
          <dd>{selected.index}</dd>
        </div>
        <div>
          <dt>Start</dt>
          <dd>{selected.span.start}</dd>
        </div>
        <div>
          <dt>End</dt>
          <dd>{selected.span.end}</dd>
        </div>
      </dl>
      <div className={selectedTextClass}>{selected.span.text}</div>
    </section>
  );
}

function DataTable({ children }: { children: ReactNode }) {
  return (
    <div className={tableWrapClass}>
      <table className={dataTableClass}>{children}</table>
    </div>
  );
}

function classNames(...values: Array<string | false | null | undefined>) {
  return values.filter(Boolean).join(" ");
}
