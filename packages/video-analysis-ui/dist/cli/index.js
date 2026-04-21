import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { Badge, EmptyState, Panel } from "../shared/primitives";
export function CliRunPanel({ run }) {
    return (_jsx(Panel, { title: "CLI Run", description: _jsxs("span", { className: "font-mono text-xs", children: [run.command, " ", (run.args ?? []).join(" ")] }), actions: _jsx(Badge, { tone: toneForStatus(run.status, run.exit_code), children: labelForStatus(run) }), children: _jsxs("div", { className: "space-y-4", children: [run.message && _jsx("p", { className: "text-sm text-zinc-700", children: run.message }), _jsxs("div", { children: [_jsx("div", { className: "mb-2 text-xs font-medium uppercase text-zinc-500", children: "Arguments" }), run.args && run.args.length > 0 ? (_jsx("div", { className: "flex flex-wrap gap-2", children: run.args.map((arg, index) => (_jsx("code", { className: "rounded-md bg-zinc-100 px-2 py-1 text-xs text-zinc-800", children: arg }, `${arg}-${index}`))) })) : (_jsx(EmptyState, { children: "No arguments" }))] }), _jsxs("div", { children: [_jsx("div", { className: "mb-2 text-xs font-medium uppercase text-zinc-500", children: "Output files" }), run.output_files && run.output_files.length > 0 ? (_jsx("ul", { className: "space-y-2", children: run.output_files.map((file) => (_jsx("li", { className: "break-all rounded-lg border border-zinc-200 px-3 py-2 text-sm text-zinc-700", children: file }, file))) })) : (_jsx(EmptyState, { children: "No output files" }))] })] }) }));
}
function labelForStatus(run) {
    if (run.exit_code != null) {
        return `exit ${run.exit_code}`;
    }
    return run.status ?? "unknown";
}
function toneForStatus(status, exitCode) {
    if (exitCode === 0 || status === "succeeded") {
        return "emerald";
    }
    if (typeof exitCode === "number" && exitCode !== 0) {
        return "rose";
    }
    if (status === "running") {
        return "sky";
    }
    if (status === "pending") {
        return "amber";
    }
    return "neutral";
}
//# sourceMappingURL=index.js.map