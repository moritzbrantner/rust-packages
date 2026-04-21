import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { VideoSummaryCards, ScenePanel, ObservationList, EventList } from "../core";
import { DataBucketOverview } from "../data";
import { SourceSummary, AssetSummary } from "../ingest";
import { CapabilityPanel, ModelObservationGrid } from "../models";
import { ReportShell } from "../output";
import { SplitPlanTable } from "../split";
import { Panel, EmptyState } from "../shared/primitives";
import { formatNumber, formatSeconds } from "../shared/utils";
export function YoutubeVideoReportView({ report }) {
    return (_jsxs(ReportShell, { title: "Video Analysis", subtitle: report.source.local_video, children: [_jsx(VideoSummaryCards, { video: report.video }), _jsxs("div", { className: "grid gap-4 lg:grid-cols-2", children: [_jsx(SourceSummary, { source: report.source }), _jsx(AssetSummary, { assets: report.assets })] }), _jsx(CapabilityPanel, { capabilities: report.capabilities }), _jsx(ScenePanel, { scenes: report.video.scenes }), _jsxs("div", { className: "grid gap-4 xl:grid-cols-2", children: [_jsx(ObservationList, { observations: report.video.observations, title: "Video Observations" }), _jsx(ModelObservationGrid, { observations: report.video.observations })] }), _jsx(TranscriptPanel, { transcription: report.transcription }), _jsxs("div", { className: "grid gap-4 xl:grid-cols-2", children: [_jsx(EventList, { events: report.audio.events, title: `Audio Events (${report.audio.status})` }), _jsx(EventList, { events: report.text.events, title: `Text Events (${report.text.status})` })] }), _jsx(DataBucketOverview, { buckets: report.data_buckets }), _jsx(SplitPlanTable, { scenes: report.video.scenes, videoName: basename(report.source.local_video) })] }));
}
export const AnalysisDashboard = YoutubeVideoReportView;
export function TranscriptPanel({ transcription, }) {
    return (_jsx(Panel, { title: `Transcript (${transcription.status})`, description: `${formatNumber(transcription.segments.length)} segments`, children: transcription.segments.length === 0 ? (_jsx(EmptyState, { children: transcription.message ?? "No transcript segments" })) : (_jsx("div", { className: "space-y-3", children: transcription.segments.map((segment) => (_jsxs("div", { className: "rounded-lg border border-zinc-200 p-3", children: [_jsxs("div", { className: "mb-2 flex flex-wrap items-center gap-2 text-xs text-zinc-500", children: [_jsxs("span", { children: ["#", segment.index] }), _jsxs("span", { children: [formatSeconds(segment.start_seconds), "-", formatSeconds(segment.end_seconds)] })] }), _jsx("p", { className: "text-sm leading-6 text-zinc-800", children: segment.text })] }, segment.index))) })) }));
}
function basename(path) {
    const last = path.split(/[\\/]/).pop() ?? path;
    return last.replace(/\.[^.]+$/, "") || "video";
}
//# sourceMappingURL=index.js.map