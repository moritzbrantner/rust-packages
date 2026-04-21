import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { Badge, EmptyState, Panel, ScoreMeter } from "../shared/primitives";
import { formatNumber } from "../shared/utils";
export function CapabilityPanel({ capabilities }) {
    return (_jsx(Panel, { title: "Capabilities", children: _jsxs("div", { className: "grid gap-4 sm:grid-cols-2", children: [_jsx(CapabilityList, { title: "Completed", items: capabilities.completed, tone: "emerald" }), _jsx(CapabilityList, { title: "Skipped", items: capabilities.skipped, tone: "amber" })] }) }));
}
export function ModelObservationGrid({ observations }) {
    const modelObservations = observations.filter((observation) => observation.score != null);
    return (_jsx(Panel, { title: "Model Observations", description: `${formatNumber(modelObservations.length)} scored`, children: modelObservations.length === 0 ? (_jsx(EmptyState, { children: "No scored model observations" })) : (_jsx("div", { className: "grid gap-3 md:grid-cols-2", children: modelObservations.map((observation, index) => (_jsxs("div", { className: "rounded-lg border border-zinc-200 p-3", children: [_jsxs("div", { className: "flex items-start justify-between gap-3", children: [_jsxs("div", { className: "min-w-0", children: [_jsx("div", { className: "truncate font-medium text-zinc-950", children: observation.label ?? observation.text ?? observation.kind }), _jsx("div", { className: "mt-1 text-sm text-zinc-500", children: observation.analyzer })] }), _jsx(Badge, { tone: "violet", children: observation.kind })] }), _jsx("div", { className: "mt-3", children: _jsx(ScoreMeter, { value: observation.score }) })] }, `${observation.analyzer}-${observation.label ?? index}-${index}`))) })) }));
}
function CapabilityList({ title, items, tone, }) {
    return (_jsxs("div", { children: [_jsx("div", { className: "mb-2 text-xs font-medium uppercase text-zinc-500", children: title }), items.length === 0 ? (_jsx(EmptyState, { children: "None" })) : (_jsx("div", { className: "flex flex-wrap gap-2", children: items.map((item) => (_jsx(Badge, { tone: tone, children: item }, item))) }))] }));
}
//# sourceMappingURL=index.js.map