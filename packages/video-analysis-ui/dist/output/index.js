import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { useState } from "react";
import { Input } from "@moritzbrantner/ui";
import { Panel } from "../shared/primitives";
export function ReportShell({ title = "Analysis Results", subtitle, actions, children, }) {
    return (_jsx("main", { className: "min-h-screen bg-zinc-50 text-zinc-950", children: _jsxs("div", { className: "mx-auto flex w-full max-w-7xl flex-col gap-4 px-4 py-6 sm:px-6 lg:px-8", children: [_jsxs("header", { className: "flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between", children: [_jsxs("div", { children: [_jsx("h1", { className: "text-2xl font-semibold tracking-normal text-zinc-950", children: title }), subtitle && _jsx("p", { className: "mt-1 text-sm text-zinc-600", children: subtitle })] }), actions && _jsx("div", { className: "flex flex-wrap gap-2", children: actions })] }), children] }) }));
}
export function JsonReportLoader({ onLoad, label = "Load JSON report", }) {
    const [error, setError] = useState(null);
    return (_jsxs(Panel, { title: label, children: [_jsx(Input, { className: "block w-full rounded-lg border border-zinc-300 bg-white px-3 py-2 text-sm text-zinc-950 file:mr-4 file:rounded-md file:border-0 file:bg-zinc-950 file:px-3 file:py-1.5 file:text-sm file:font-medium file:text-white", type: "file", accept: "application/json,.json", onChange: (event) => {
                    const file = event.currentTarget.files?.[0];
                    if (!file) {
                        return;
                    }
                    file
                        .text()
                        .then((text) => {
                        onLoad(JSON.parse(text));
                        setError(null);
                    })
                        .catch((nextError) => {
                        setError(nextError instanceof Error ? nextError.message : "Invalid JSON report");
                    });
                } }), error && _jsx("p", { className: "mt-2 text-sm text-rose-700", children: error })] }));
}
//# sourceMappingURL=index.js.map