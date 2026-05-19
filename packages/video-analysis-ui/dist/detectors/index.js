import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { DataTable, EmptyState, Panel, StatCard } from "../shared/primitives";
import { formatNumber, formatScore, formatSeconds, timestampSeconds } from "../shared/utils";
export function DetectionSummary({ result, detector, }) {
    return (_jsxs(Panel, { title: "Detection", description: detector, children: [_jsxs("div", { className: "grid gap-3 sm:grid-cols-3", children: [_jsx(StatCard, { label: "Scenes", value: formatNumber(result.scenes.length), tone: "sky" }), _jsx(StatCard, { label: "Cuts", value: formatNumber(result.cuts?.length ?? 0), tone: "emerald" }), _jsx(StatCard, { label: "Frames", value: formatNumber(result.frames_processed), tone: "amber" })] }), result.cuts && result.cuts.length > 0 && (_jsx("div", { className: "mt-4", children: _jsx(CutTable, { cuts: result.cuts }) }))] }));
}
export function CutTable({ cuts }) {
    if (cuts.length === 0) {
        return _jsx(EmptyState, { children: "No cuts" });
    }
    return (_jsx(DataTable, { rows: cuts, getRowKey: (cut, index) => `${cut.position.frame_index}-${cut.detector}-${index}`, columns: [
            {
                key: "frame",
                header: "Frame",
                className: "tabular-nums text-zinc-700",
                cell: (cut) => formatNumber(cut.position.frame_index),
            },
            {
                key: "time",
                header: "Time",
                className: "tabular-nums text-zinc-700",
                cell: (cut) => formatSeconds(timestampSeconds(cut.position.timestamp)),
            },
            {
                key: "detector",
                header: "Detector",
                className: "font-medium text-zinc-950",
                cell: (cut) => cut.detector,
            },
            {
                key: "score",
                header: "Score",
                className: "text-zinc-700",
                cell: (cut) => formatScore(cut.score),
            },
        ] }));
}
//# sourceMappingURL=index.js.map