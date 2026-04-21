import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { Panel, StatCard } from "../shared/primitives";
import { formatSeconds } from "../shared/utils";
export function MediaMetadataPanel({ metadata }) {
    return (_jsx(Panel, { title: "Media Metadata", description: metadata.input, children: _jsxs("div", { className: "grid gap-3 sm:grid-cols-2 lg:grid-cols-4", children: [_jsx(StatCard, { label: "Mode", value: metadata.mode ?? "n/a", tone: "sky" }), _jsx(StatCard, { label: "Video", value: metadata.width && metadata.height ? `${metadata.width}x${metadata.height}` : "n/a", detail: metadata.frame_rate ?? undefined, tone: "emerald" }), _jsx(StatCard, { label: "Duration", value: formatSeconds(metadata.duration_seconds), tone: "amber" }), _jsx(StatCard, { label: "Audio", value: metadata.sample_rate ? `${metadata.sample_rate} Hz` : "n/a", detail: metadata.channels ? `${metadata.channels} channels` : undefined, tone: "violet" })] }) }));
}
//# sourceMappingURL=index.js.map