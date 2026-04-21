import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { cn } from "./utils";
const toneClasses = {
    neutral: "border-zinc-200 bg-white text-zinc-700",
    sky: "border-sky-200 bg-sky-50 text-sky-800",
    emerald: "border-emerald-200 bg-emerald-50 text-emerald-800",
    amber: "border-amber-200 bg-amber-50 text-amber-800",
    rose: "border-rose-200 bg-rose-50 text-rose-800",
    violet: "border-violet-200 bg-violet-50 text-violet-800",
};
export function Panel({ title, description, actions, children, className, }) {
    return (_jsxs("section", { className: cn("rounded-lg border border-zinc-200 bg-white shadow-sm", className), children: [(title || description || actions) && (_jsxs("div", { className: "flex flex-col gap-3 border-b border-zinc-200 px-4 py-3 sm:flex-row sm:items-center sm:justify-between", children: [_jsxs("div", { children: [title && _jsx("h2", { className: "text-sm font-semibold text-zinc-950", children: title }), description && _jsx("p", { className: "mt-1 text-sm text-zinc-600", children: description })] }), actions && _jsx("div", { className: "flex items-center gap-2", children: actions })] })), _jsx("div", { className: "p-4", children: children })] }));
}
export function Badge({ children, tone = "neutral", className, }) {
    return (_jsx("span", { className: cn("inline-flex min-h-6 items-center rounded-md border px-2 py-0.5 text-xs font-medium", toneClasses[tone], className), children: children }));
}
export function StatCard({ label, value, detail, tone = "neutral", }) {
    return (_jsxs("div", { className: cn("rounded-lg border p-3", toneClasses[tone]), children: [_jsx("div", { className: "text-xs font-medium uppercase tracking-normal opacity-75", children: label }), _jsx("div", { className: "mt-1 text-xl font-semibold text-zinc-950", children: value }), detail && _jsx("div", { className: "mt-1 text-xs opacity-75", children: detail })] }));
}
export function EmptyState({ children = "No results" }) {
    return (_jsx("div", { className: "flex min-h-24 items-center justify-center rounded-lg border border-dashed border-zinc-300 bg-zinc-50 px-4 py-6 text-sm text-zinc-500", children: children }));
}
export function ScoreMeter({ value }) {
    const normalized = value == null ? 0 : value <= 1 ? value * 100 : Math.min(value, 100);
    return (_jsxs("div", { className: "flex min-w-28 items-center gap-2", children: [_jsx("div", { className: "h-2 w-20 overflow-hidden rounded-full bg-zinc-200", children: _jsx("div", { className: "h-full rounded-full bg-emerald-500", style: { width: `${normalized}%` } }) }), _jsx("span", { className: "w-12 text-right text-xs tabular-nums text-zinc-600", children: value == null ? "n/a" : value <= 1 ? `${Math.round(value * 100)}%` : value.toFixed(1) })] }));
}
//# sourceMappingURL=primitives.js.map