import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { EmptyState, Panel } from "../shared/primitives";
import { formatSeconds, sceneEndFrame, sceneEndSeconds, sceneIndex, sceneStartFrame, sceneStartSeconds, } from "../shared/utils";
export function SplitPlanTable({ scenes, videoName = "video", template = "$VIDEO_NAME-Scene-$SCENE_NUMBER.mp4", }) {
    return (_jsx(Panel, { title: "Split Plan", description: `${scenes.length} output clips`, children: scenes.length === 0 ? (_jsx(EmptyState, { children: "No scene clips" })) : (_jsx("div", { className: "overflow-x-auto", children: _jsxs("table", { className: "min-w-full text-left text-sm", children: [_jsx("thead", { className: "border-b border-zinc-200 text-xs uppercase text-zinc-500", children: _jsxs("tr", { children: [_jsx("th", { className: "px-3 py-2 font-medium", children: "Output" }), _jsx("th", { className: "px-3 py-2 font-medium", children: "Start" }), _jsx("th", { className: "px-3 py-2 font-medium", children: "Duration" }), _jsx("th", { className: "px-3 py-2 font-medium", children: "Frames" })] }) }), _jsx("tbody", { className: "divide-y divide-zinc-100", children: scenes.map((scene, index) => {
                            const number = sceneIndex(scene, index + 1);
                            const start = sceneStartSeconds(scene);
                            const end = sceneEndSeconds(scene);
                            return (_jsxs("tr", { children: [_jsx("td", { className: "px-3 py-2 font-medium text-zinc-950", children: formatOutputName(template, videoName, number, scene) }), _jsx("td", { className: "px-3 py-2 tabular-nums text-zinc-700", children: formatSeconds(start) }), _jsx("td", { className: "px-3 py-2 tabular-nums text-zinc-700", children: formatSeconds(end - start) }), _jsxs("td", { className: "px-3 py-2 tabular-nums text-zinc-700", children: [sceneStartFrame(scene), "-", sceneEndFrame(scene)] })] }, `${sceneStartFrame(scene)}-${sceneEndFrame(scene)}-${index}`));
                        }) })] }) })) }));
}
function formatOutputName(template, videoName, sceneNumber, scene) {
    const digits = Math.max(3, String(sceneNumber).length);
    return template
        .split("$VIDEO_NAME")
        .join(videoName)
        .split("$SCENE_NUMBER")
        .join(String(sceneNumber).padStart(digits, "0"))
        .split("$START_FRAME")
        .join(String(sceneStartFrame(scene)))
        .split("$END_FRAME")
        .join(String(sceneEndFrame(scene)));
}
//# sourceMappingURL=index.js.map