import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { Badge, EmptyState, Panel, ScoreMeter, StatCard } from "../shared/primitives";
import { cn, formatNumber, formatScore, formatSeconds, sceneEndFrame, sceneEndSeconds, sceneIndex, sceneStartFrame, sceneStartSeconds, } from "../shared/utils";
export function VideoSummaryCards({ video }) {
    return (_jsxs("div", { className: "grid gap-3 sm:grid-cols-2 lg:grid-cols-4", children: [_jsx(StatCard, { label: "Resolution", value: `${video.width}x${video.height}`, detail: video.frame_rate, tone: "sky" }), _jsx(StatCard, { label: "Duration", value: formatSeconds(video.duration_seconds), tone: "emerald" }), _jsx(StatCard, { label: "Frames", value: formatNumber(video.frames_processed), tone: "amber" }), _jsx(StatCard, { label: "Scenes", value: formatNumber(video.scenes.length), tone: "violet" })] }));
}
export function SceneTimeline({ scenes, durationSeconds, activeSceneIndex, onSelectScene, className, }) {
    if (scenes.length === 0) {
        return _jsx(EmptyState, { children: "No scenes detected" });
    }
    const computedDuration = durationSeconds ?? Math.max(...scenes.map((scene) => sceneEndSeconds(scene)), 0);
    return (_jsxs("div", { className: cn("space-y-3", className), children: [_jsx("div", { className: "flex h-16 overflow-hidden rounded-lg border border-zinc-200 bg-zinc-100", children: scenes.map((scene, index) => {
                    const start = sceneStartSeconds(scene);
                    const end = sceneEndSeconds(scene);
                    const width = Math.max(end - start, computedDuration / 200, 0.01);
                    const selected = activeSceneIndex === sceneIndex(scene, index + 1);
                    const Component = onSelectScene ? "button" : "div";
                    return (_jsx(Component, { className: cn("group relative min-w-4 border-r border-white/70 bg-sky-500 text-left outline-none transition-colors last:border-r-0 hover:bg-sky-600 focus-visible:ring-2 focus-visible:ring-sky-400 focus-visible:ring-offset-2", index % 2 === 1 && "bg-emerald-500 hover:bg-emerald-600", selected && "bg-zinc-950 hover:bg-zinc-900"), style: { flexGrow: width, flexBasis: 0 }, title: `Scene ${sceneIndex(scene, index + 1)} ${formatSeconds(start)}-${formatSeconds(end)}`, onClick: () => onSelectScene?.(scene, index), children: _jsx("span", { className: "absolute inset-x-1 bottom-1 truncate text-[11px] font-medium text-white", children: sceneIndex(scene, index + 1) }) }, `${sceneStartFrame(scene)}-${sceneEndFrame(scene)}-${index}`));
                }) }), _jsxs("div", { className: "flex items-center justify-between text-xs text-zinc-500", children: [_jsx("span", { children: formatSeconds(0) }), _jsx("span", { children: formatSeconds(computedDuration) })] })] }));
}
export function SceneTable({ scenes, onSelectScene, }) {
    if (scenes.length === 0) {
        return _jsx(EmptyState, { children: "No scene rows" });
    }
    return (_jsx("div", { className: "overflow-x-auto", children: _jsxs("table", { className: "min-w-full text-left text-sm", children: [_jsx("thead", { className: "border-b border-zinc-200 text-xs uppercase text-zinc-500", children: _jsxs("tr", { children: [_jsx("th", { className: "px-3 py-2 font-medium", children: "Scene" }), _jsx("th", { className: "px-3 py-2 font-medium", children: "Start" }), _jsx("th", { className: "px-3 py-2 font-medium", children: "End" }), _jsx("th", { className: "px-3 py-2 font-medium", children: "Frames" }), _jsx("th", { className: "px-3 py-2 font-medium", children: "Duration" })] }) }), _jsx("tbody", { className: "divide-y divide-zinc-100", children: scenes.map((scene, index) => {
                        const start = sceneStartSeconds(scene);
                        const end = sceneEndSeconds(scene);
                        return (_jsxs("tr", { className: cn(onSelectScene && "cursor-pointer hover:bg-zinc-50"), onClick: () => onSelectScene?.(scene, index), children: [_jsx("td", { className: "px-3 py-2 font-medium text-zinc-950", children: sceneIndex(scene, index + 1) }), _jsx("td", { className: "px-3 py-2 tabular-nums text-zinc-700", children: formatSeconds(start) }), _jsx("td", { className: "px-3 py-2 tabular-nums text-zinc-700", children: formatSeconds(end) }), _jsxs("td", { className: "px-3 py-2 tabular-nums text-zinc-700", children: [formatNumber(sceneStartFrame(scene)), "-", formatNumber(sceneEndFrame(scene))] }), _jsx("td", { className: "px-3 py-2 tabular-nums text-zinc-700", children: formatSeconds(end - start) })] }, `${sceneStartFrame(scene)}-${sceneEndFrame(scene)}-${index}`));
                    }) })] }) }));
}
export function ObservationList({ observations, title = "Observations", }) {
    return (_jsx(Panel, { title: title, description: `${formatNumber(observations.length)} results`, children: observations.length === 0 ? (_jsx(EmptyState, { children: "No observations" })) : (_jsx("div", { className: "space-y-3", children: observations.map((observation, index) => (_jsxs("div", { className: "grid gap-3 rounded-lg border border-zinc-200 p-3 sm:grid-cols-[1fr_auto]", children: [_jsxs("div", { className: "min-w-0", children: [_jsxs("div", { className: "flex flex-wrap items-center gap-2", children: [_jsx(Badge, { tone: toneForKind(observation.kind), children: observation.kind }), _jsx("span", { className: "text-sm font-medium text-zinc-950", children: observation.label ?? observation.text ?? "Unlabeled" }), _jsx("span", { className: "text-xs text-zinc-500", children: observation.analyzer })] }), observation.text && observation.text !== observation.label && (_jsx("p", { className: "mt-2 line-clamp-3 text-sm text-zinc-700", children: observation.text })), _jsxs("div", { className: "mt-2 flex flex-wrap gap-2 text-xs text-zinc-500", children: [_jsx("span", { children: formatSeconds(observation.timestamp_seconds) }), observation.frame_index != null && _jsxs("span", { children: ["frame ", formatNumber(observation.frame_index)] }), observation.scene_index != null && _jsxs("span", { children: ["scene ", formatNumber(observation.scene_index)] }), observation.region && (_jsxs("span", { children: ["box ", observation.region.x, ",", observation.region.y, " ", observation.region.width, "x", observation.region.height] }))] })] }), _jsx(ScoreMeter, { value: observation.score })] }, `${observation.analyzer}-${observation.kind}-${observation.frame_index ?? index}-${index}`))) })) }));
}
export function EventList({ events, title = "Events", empty = "No events", }) {
    return (_jsx(Panel, { title: title, description: `${formatNumber(events.length)} results`, children: events.length === 0 ? (_jsx(EmptyState, { children: empty })) : (_jsx("div", { className: "overflow-x-auto", children: _jsxs("table", { className: "min-w-full text-left text-sm", children: [_jsx("thead", { className: "border-b border-zinc-200 text-xs uppercase text-zinc-500", children: _jsxs("tr", { children: [_jsx("th", { className: "px-3 py-2 font-medium", children: "Time" }), _jsx("th", { className: "px-3 py-2 font-medium", children: "Analyzer" }), _jsx("th", { className: "px-3 py-2 font-medium", children: "Label" }), _jsx("th", { className: "px-3 py-2 font-medium", children: "Score" })] }) }), _jsx("tbody", { className: "divide-y divide-zinc-100", children: events.map((event, index) => (_jsxs("tr", { children: [_jsx("td", { className: "px-3 py-2 tabular-nums text-zinc-700", children: formatSeconds(event.timestamp_seconds) }), _jsx("td", { className: "px-3 py-2 text-zinc-700", children: event.analyzer }), _jsx("td", { className: "px-3 py-2 font-medium text-zinc-950", children: event.label }), _jsx("td", { className: "px-3 py-2 text-zinc-700", children: formatScore(event.score) })] }, `${event.analyzer}-${event.label}-${event.timestamp_seconds ?? index}-${index}`))) })] }) })) }));
}
export function ScenePanel({ scenes }) {
    return (_jsxs(Panel, { title: "Scenes", description: `${formatNumber(scenes.length)} detected`, children: [_jsx(SceneTimeline, { scenes: scenes }), _jsx("div", { className: "mt-4", children: _jsx(SceneTable, { scenes: scenes }) })] }));
}
function toneForKind(kind) {
    const normalized = kind.toLowerCase();
    if (normalized.includes("object"))
        return "sky";
    if (normalized.includes("text"))
        return "emerald";
    if (normalized.includes("face"))
        return "amber";
    if (normalized.includes("scene"))
        return "violet";
    return "neutral";
}
//# sourceMappingURL=index.js.map