import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { DataTable, EmptyState, Panel, StatCard } from "../shared/primitives";
import { formatBytes, formatNumber, ratioPercent } from "../shared/utils";
export function DataBucketOverview({ buckets }) {
    const totalRecords = buckets.reduce((sum, bucket) => sum + bucket.records, 0);
    const totalBytes = buckets.reduce((sum, bucket) => sum + bucket.estimated_bytes, 0);
    const maxRecords = Math.max(...buckets.map((bucket) => bucket.records), 0);
    return (_jsx(Panel, { title: "Data Buckets", description: `${formatNumber(buckets.length)} summaries`, children: buckets.length === 0 ? (_jsx(EmptyState, { children: "No data buckets" })) : (_jsxs("div", { className: "space-y-4", children: [_jsxs("div", { className: "grid gap-3 sm:grid-cols-3", children: [_jsx(StatCard, { label: "Buckets", value: formatNumber(buckets.length), tone: "sky" }), _jsx(StatCard, { label: "Records", value: formatNumber(totalRecords), tone: "emerald" }), _jsx(StatCard, { label: "Estimated bytes", value: formatBytes(totalBytes), tone: "amber" })] }), _jsx("div", { className: "space-y-2", children: buckets.map((bucket) => (_jsxs("div", { className: "rounded-lg border border-zinc-200 p-3", children: [_jsxs("div", { className: "flex flex-wrap items-center justify-between gap-2", children: [_jsxs("div", { className: "font-medium text-zinc-950", children: ["Bucket ", bucket.bucket_index] }), _jsxs("div", { className: "text-sm text-zinc-500", children: [formatNumber(bucket.records), " records, ", formatBytes(bucket.estimated_bytes)] })] }), _jsx("div", { className: "mt-2 h-2 overflow-hidden rounded-full bg-zinc-200", children: _jsx("div", { className: "h-full rounded-full bg-sky-500", style: { width: `${ratioPercent(bucket.records, maxRecords)}%` } }) }), _jsx("div", { className: "mt-3", children: _jsx(StreamSummaryTable, { streams: bucket.streams, compact: true }) })] }, bucket.bucket_index))) })] })) }));
}
export function StreamSummaryTable({ streams, compact = false, }) {
    if (streams.length === 0) {
        return _jsx(EmptyState, { children: "No streams" });
    }
    return (_jsx(DataTable, { rows: streams, getRowKey: (stream) => stream.stream_id, columns: [
            {
                key: "stream",
                header: "Stream",
                className: "font-medium text-zinc-950",
                cell: (stream) => stream.stream_id,
            },
            {
                key: "records",
                header: "Records",
                className: "tabular-nums text-zinc-700",
                cell: (stream) => formatNumber(stream.records),
            },
            {
                key: "bytes",
                header: "Bytes",
                className: "tabular-nums text-zinc-700",
                cell: (stream) => formatBytes(stream.estimated_bytes),
            },
            ...(!compact
                ? [
                    {
                        key: "payloads",
                        header: "Payloads",
                        className: "text-zinc-700",
                        cell: (stream) => payloadSummary(stream.payload_counts),
                    },
                ]
                : []),
            {
                key: "video",
                header: "Video",
                className: "tabular-nums text-zinc-700",
                cell: (stream) => formatNumber(stream.video_frames),
            },
            {
                key: "audio",
                header: "Audio",
                className: "tabular-nums text-zinc-700",
                cell: (stream) => formatNumber(stream.audio_frames),
            },
            {
                key: "text",
                header: "Text",
                className: "tabular-nums text-zinc-700",
                cell: (stream) => formatNumber(stream.text_segments),
            },
        ] }));
}
function payloadSummary(payloads) {
    const entries = Object.entries(payloads);
    if (entries.length === 0) {
        return "n/a";
    }
    return entries.map(([key, value]) => `${key}: ${formatNumber(value)}`).join(", ");
}
//# sourceMappingURL=index.js.map