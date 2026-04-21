import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { Badge, Panel } from "../shared/primitives";
export function SourceSummary({ source }) {
    return (_jsx(Panel, { title: "Source", children: _jsxs("dl", { className: "grid gap-3 text-sm", children: [_jsxs("div", { children: [_jsx("dt", { className: "text-xs uppercase text-zinc-500", children: "Input" }), _jsx("dd", { className: "mt-1 break-all font-medium text-zinc-950", children: source.local_video })] }), source.url && (_jsxs("div", { children: [_jsx("dt", { className: "text-xs uppercase text-zinc-500", children: "URL" }), _jsx("dd", { className: "mt-1 break-all text-zinc-700", children: source.url })] }))] }) }));
}
export function AssetSummary({ assets }) {
    return (_jsx(Panel, { title: "Assets", children: _jsxs("div", { className: "space-y-3 text-sm", children: [_jsx(AssetRow, { label: "Work dir", value: assets.work_dir }), _jsx(AssetRow, { label: "Report", value: assets.report_path }), _jsx(AssetRow, { label: "Audio", value: assets.audio_wav ?? "not generated", muted: !assets.audio_wav })] }) }));
}
function AssetRow({ label, value, muted = false }) {
    return (_jsxs("div", { className: "flex flex-col gap-1 sm:flex-row sm:items-center sm:justify-between", children: [_jsx(Badge, { tone: muted ? "neutral" : "sky", children: label }), _jsx("span", { className: muted ? "text-zinc-500" : "break-all text-zinc-700", children: value })] }));
}
//# sourceMappingURL=index.js.map