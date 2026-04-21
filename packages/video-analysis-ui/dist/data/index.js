import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { EmptyState, Panel, StatCard } from "../shared/primitives";
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
    return (_jsx("div", { className: "overflow-x-auto", children: _jsxs("table", { className: "min-w-full text-left text-sm", children: [_jsx("thead", { className: "border-b border-zinc-200 text-xs uppercase text-zinc-500", children: _jsxs("tr", { children: [_jsx("th", { className: "px-3 py-2 font-medium", children: "Stream" }), _jsx("th", { className: "px-3 py-2 font-medium", children: "Records" }), _jsx("th", { className: "px-3 py-2 font-medium", children: "Bytes" }), !compact && _jsx("th", { className: "px-3 py-2 font-medium", children: "Payloads" }), _jsx("th", { className: "px-3 py-2 font-medium", children: "Video" }), _jsx("th", { className: "px-3 py-2 font-medium", children: "Audio" }), _jsx("th", { className: "px-3 py-2 font-medium", children: "Text" })] }) }), _jsx("tbody", { className: "divide-y divide-zinc-100", children: streams.map((stream) => (_jsxs("tr", { children: [_jsx("td", { className: "px-3 py-2 font-medium text-zinc-950", children: stream.stream_id }), _jsx("td", { className: "px-3 py-2 tabular-nums text-zinc-700", children: formatNumber(stream.records) }), _jsx("td", { className: "px-3 py-2 tabular-nums text-zinc-700", children: formatBytes(stream.estimated_bytes) }), !compact && (_jsx("td", { className: "px-3 py-2 text-zinc-700", children: payloadSummary(stream.payload_counts) })), _jsx("td", { className: "px-3 py-2 tabular-nums text-zinc-700", children: formatNumber(stream.video_frames) }), _jsx("td", { className: "px-3 py-2 tabular-nums text-zinc-700", children: formatNumber(stream.audio_frames) }), _jsx("td", { className: "px-3 py-2 tabular-nums text-zinc-700", children: formatNumber(stream.text_segments) })] }, stream.stream_id))) })] }) }));
}
function payloadSummary(payloads) {
    const entries = Object.entries(payloads);
    if (entries.length === 0) {
        return "n/a";
    }
    return entries.map(([key, value]) => `${key}: ${formatNumber(value)}`).join(", ");
}
//# sourceMappingURL=index.js.map