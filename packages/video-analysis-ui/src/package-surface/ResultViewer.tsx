import { useMemo, useState } from "react";

import type { ResultTabDefinition, SurfaceResponse } from "./types";

type ResultTab = ResultTabDefinition & { description?: string };

export function ResultViewer({
  response,
  resultTabs = [],
}: {
  response: SurfaceResponse | null;
  resultTabs?: ResultTabDefinition[];
}) {
  const tabs = useMemo<ResultTab[]>(
    () => [
      {
        id: "summary",
        label: "Summary",
        description: "Compact response summary",
        select: defaultSummary,
      },
      {
        id: "json",
        label: "JSON",
        description: "Full raw response JSON",
        select: (value: SurfaceResponse) => value,
      },
      {
        id: "diagnostics",
        label: "Diagnostics",
        description: "Warnings, notes, and non-fatal operation messages",
        select: (value: SurfaceResponse) => value.diagnostics,
      },
      {
        id: "artifacts",
        label: "Artifacts",
        description: "Generated outputs and file-like references",
        select: (value: SurfaceResponse) => value.artifacts,
      },
      ...resultTabs,
    ],
    [resultTabs],
  );
  const [activeTab, setActiveTab] = useState(tabs[0]?.id ?? "summary");
  const tab = tabs.find((candidate) => candidate.id === activeTab) ?? tabs[0];
  const customRendered = tab?.render ? tab.render(response) : null;
  const selected = response && tab?.select ? tab.select(response) : (response ?? {});
  const rendered = JSON.stringify(selected, null, 2);

  return (
    <section className="rounded-md border border-zinc-200 bg-white p-4">
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-zinc-200 pb-3">
        <div className="flex flex-wrap gap-2">
          {tabs.map((candidate) => (
            <button
              key={candidate.id}
              className={
                activeTab === candidate.id
                  ? "rounded-md bg-zinc-950 px-3 py-2 text-sm font-semibold text-white"
                  : "rounded-md bg-zinc-100 px-3 py-2 text-sm font-semibold text-zinc-700 hover:bg-zinc-200"
              }
              aria-label={candidate.description ? `${candidate.label}: ${candidate.description}` : candidate.label}
              title={candidate.description}
              type="button"
              onClick={() => setActiveTab(candidate.id)}
            >
              {candidate.label}
            </button>
          ))}
        </div>
        {tab?.render ? null : (
          <div className="flex gap-2">
            <button className="rounded-md border border-zinc-300 px-3 py-2 text-sm font-semibold" type="button" onClick={() => void copyText(rendered)}>
              Copy
            </button>
            <button className="rounded-md border border-zinc-300 px-3 py-2 text-sm font-semibold" type="button" onClick={() => downloadJson(rendered)}>
              Download
            </button>
          </div>
        )}
      </div>
      {customRendered ?? (
        <pre className="mt-4 max-h-[42rem] overflow-auto rounded-md bg-zinc-950 p-4 text-sm leading-6 text-zinc-50">
          {rendered}
        </pre>
      )}
    </section>
  );
}

function defaultSummary(response: SurfaceResponse): unknown {
  const value = response.value;
  const object = value && typeof value === "object" && !Array.isArray(value) ? (value as Record<string, unknown>) : {};
  return {
    operation: response.operation,
    title: typeof object.title === "string" ? object.title : undefined,
    message: typeof object.message === "string" ? object.message : undefined,
    summary: object.summary,
    diagnostics: response.diagnostics.length,
    artifacts: response.artifacts.length,
    keys: Object.keys(object).slice(0, 16),
    value,
  };
}

async function copyText(text: string) {
  await navigator.clipboard?.writeText(text);
}

function downloadJson(text: string) {
  const blob = new Blob([text], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = "surface-response.json";
  anchor.click();
  URL.revokeObjectURL(url);
}
