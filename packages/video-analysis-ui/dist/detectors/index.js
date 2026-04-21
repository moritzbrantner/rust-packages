import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { EmptyState, Panel, StatCard } from "../shared/primitives";
import { formatNumber, formatScore, formatSeconds, timestampSeconds } from "../shared/utils";
export function DetectionSummary({ result, detector, }) {
    return (_jsxs(Panel, { title: "Detection", description: detector, children: [_jsxs("div", { className: "grid gap-3 sm:grid-cols-3", children: [_jsx(StatCard, { label: "Scenes", value: formatNumber(result.scenes.length), tone: "sky" }), _jsx(StatCard, { label: "Cuts", value: formatNumber(result.cuts?.length ?? 0), tone: "emerald" }), _jsx(StatCard, { label: "Frames", value: formatNumber(result.frames_processed), tone: "amber" })] }), result.cuts && result.cuts.length > 0 && (_jsx("div", { className: "mt-4", children: _jsx(CutTable, { cuts: result.cuts }) }))] }));
}
export function CutTable({ cuts }) {
    if (cuts.length === 0) {
        return _jsx(EmptyState, { children: "No cuts" });
    }
    return (_jsx("div", { className: "overflow-x-auto", children: _jsxs("table", { className: "min-w-full text-left text-sm", children: [_jsx("thead", { className: "border-b border-zinc-200 text-xs uppercase text-zinc-500", children: _jsxs("tr", { children: [_jsx("th", { className: "px-3 py-2 font-medium", children: "Frame" }), _jsx("th", { className: "px-3 py-2 font-medium", children: "Time" }), _jsx("th", { className: "px-3 py-2 font-medium", children: "Detector" }), _jsx("th", { className: "px-3 py-2 font-medium", children: "Score" })] }) }), _jsx("tbody", { className: "divide-y divide-zinc-100", children: cuts.map((cut, index) => (_jsxs("tr", { children: [_jsx("td", { className: "px-3 py-2 tabular-nums text-zinc-700", children: formatNumber(cut.position.frame_index) }), _jsx("td", { className: "px-3 py-2 tabular-nums text-zinc-700", children: formatSeconds(timestampSeconds(cut.position.timestamp)) }), _jsx("td", { className: "px-3 py-2 font-medium text-zinc-950", children: cut.detector }), _jsx("td", { className: "px-3 py-2 text-zinc-700", children: formatScore(cut.score) })] }, `${cut.position.frame_index}-${cut.detector}-${index}`))) })] }) }));
}
//# sourceMappingURL=index.js.map