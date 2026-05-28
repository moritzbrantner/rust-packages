import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { useMemo, useState } from "react";
export function ResultViewer({ response, resultTabs = [], }) {
    const tabs = useMemo(() => [
        { id: "summary", label: "Summary", select: defaultSummary },
        { id: "json", label: "JSON", select: (value) => value },
        { id: "diagnostics", label: "Diagnostics", select: (value) => value.diagnostics },
        { id: "artifacts", label: "Artifacts", select: (value) => value.artifacts },
        ...resultTabs,
    ], [resultTabs]);
    const [activeTab, setActiveTab] = useState(tabs[0]?.id ?? "summary");
    const tab = tabs.find((candidate) => candidate.id === activeTab) ?? tabs[0];
    const selected = response && tab ? tab.select(response) : {};
    const rendered = JSON.stringify(selected, null, 2);
    return (_jsxs("section", { className: "rounded-md border border-zinc-200 bg-white p-4", children: [_jsxs("div", { className: "flex flex-wrap items-center justify-between gap-3 border-b border-zinc-200 pb-3", children: [_jsx("div", { className: "flex flex-wrap gap-2", children: tabs.map((candidate) => (_jsx("button", { className: activeTab === candidate.id
                                ? "rounded-md bg-zinc-950 px-3 py-2 text-sm font-semibold text-white"
                                : "rounded-md bg-zinc-100 px-3 py-2 text-sm font-semibold text-zinc-700 hover:bg-zinc-200", type: "button", onClick: () => setActiveTab(candidate.id), children: candidate.label }, candidate.id))) }), _jsxs("div", { className: "flex gap-2", children: [_jsx("button", { className: "rounded-md border border-zinc-300 px-3 py-2 text-sm font-semibold", type: "button", onClick: () => void copyText(rendered), children: "Copy" }), _jsx("button", { className: "rounded-md border border-zinc-300 px-3 py-2 text-sm font-semibold", type: "button", onClick: () => downloadJson(rendered), children: "Download" })] })] }), _jsx("pre", { className: "mt-4 max-h-[42rem] overflow-auto rounded-md bg-zinc-950 p-4 text-sm leading-6 text-zinc-50", children: rendered })] }));
}
function defaultSummary(response) {
    const value = response.value;
    const object = value && typeof value === "object" && !Array.isArray(value) ? value : {};
    return {
        operation: response.operation,
        diagnostics: response.diagnostics.length,
        artifacts: response.artifacts.length,
        keys: Object.keys(object).slice(0, 16),
        value,
    };
}
async function copyText(text) {
    await navigator.clipboard?.writeText(text);
}
function downloadJson(text) {
    const blob = new Blob([text], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = "surface-response.json";
    anchor.click();
    URL.revokeObjectURL(url);
}
//# sourceMappingURL=ResultViewer.js.map