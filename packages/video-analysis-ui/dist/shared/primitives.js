import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { Badge as UiBadge, Card, CardAction, CardContent, CardDescription, CardHeader, Empty, EmptyDescription, EmptyHeader, Stat, StatDescription, StatLabel, StatValue, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, } from "@moritzbrantner/ui";
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
    return (_jsxs(Card, { className: cn("gap-0 rounded-lg border border-zinc-200 bg-white py-0 text-zinc-950 shadow-sm ring-0", className), children: [(title || description || actions) && (_jsxs(CardHeader, { className: "flex flex-col gap-3 border-b border-zinc-200 px-4 py-3 sm:flex-row sm:items-center sm:justify-between", children: [_jsxs("div", { children: [title && _jsx("h2", { className: "text-sm font-semibold text-zinc-950", children: title }), description && (_jsx(CardDescription, { className: "mt-1 text-sm text-zinc-600", children: description }))] }), actions && _jsx(CardAction, { className: "flex items-center gap-2", children: actions })] })), _jsx(CardContent, { className: "p-4", children: children })] }));
}
export function Badge({ children, tone = "neutral", className, }) {
    return (_jsx(UiBadge, { variant: "outline", className: cn("inline-flex min-h-6 rounded-md border px-2 py-0.5 text-xs font-medium shadow-none", toneClasses[tone], className), children: children }));
}
export function StatCard({ label, value, detail, tone = "neutral", }) {
    return (_jsxs(Stat, { className: cn("gap-1 rounded-lg border p-3 shadow-none", toneClasses[tone]), children: [_jsx(StatLabel, { className: "text-xs font-medium uppercase tracking-normal opacity-75", children: label }), _jsx(StatValue, { className: "mt-1 text-xl font-semibold text-zinc-950", children: value }), detail && (_jsx(StatDescription, { className: "mt-1 text-xs leading-normal opacity-75", children: detail }))] }));
}
export function EmptyState({ children = "No results" }) {
    return (_jsx(Empty, { className: "min-h-24 rounded-lg border border-dashed border-zinc-300 bg-zinc-50 px-4 py-6 text-sm text-zinc-500", children: _jsx(EmptyHeader, { children: _jsx(EmptyDescription, { className: "text-sm text-zinc-500", children: children }) }) }));
}
export function DataTable({ rows, columns, getRowKey, empty = "No rows", onRowClick, rowClassName, }) {
    if (rows.length === 0) {
        return _jsx(EmptyState, { children: empty });
    }
    const handleRowKeyDown = (event, row, index) => {
        if (!onRowClick || (event.key !== "Enter" && event.key !== " ")) {
            return;
        }
        event.preventDefault();
        onRowClick(row, index);
    };
    return (_jsxs(Table, { className: "min-w-full text-left text-sm", children: [_jsx(TableHeader, { className: "border-b border-zinc-200 text-xs uppercase text-zinc-500", children: _jsx(TableRow, { className: "border-zinc-200 hover:bg-transparent", children: columns.map((column) => (_jsx(TableHead, { className: cn("px-3 py-2 font-medium text-zinc-500", column.headerClassName), children: column.header }, column.key))) }) }), _jsx(TableBody, { className: "divide-y divide-zinc-100", children: rows.map((row, index) => (_jsx(TableRow, { className: cn("border-zinc-100 hover:bg-zinc-50", onRowClick && "cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sky-400", rowClassName?.(row, index)), tabIndex: onRowClick ? 0 : undefined, role: onRowClick ? "button" : undefined, onClick: () => onRowClick?.(row, index), onKeyDown: (event) => handleRowKeyDown(event, row, index), children: columns.map((column) => (_jsx(TableCell, { className: cn("px-3 py-2", column.className), children: column.cell(row, index) }, column.key))) }, getRowKey(row, index)))) })] }));
}
export function ScoreMeter({ value }) {
    const normalized = value == null ? 0 : value <= 1 ? value * 100 : Math.min(value, 100);
    return (_jsxs("div", { className: "flex min-w-28 items-center gap-2", children: [_jsx("div", { className: "h-2 w-20 overflow-hidden rounded-full bg-zinc-200", children: _jsx("div", { className: "h-full rounded-full bg-emerald-500", style: { width: `${normalized}%` } }) }), _jsx("span", { className: "w-12 text-right text-xs tabular-nums text-zinc-600", children: value == null ? "n/a" : value <= 1 ? `${Math.round(value * 100)}%` : value.toFixed(1) })] }));
}
//# sourceMappingURL=primitives.js.map