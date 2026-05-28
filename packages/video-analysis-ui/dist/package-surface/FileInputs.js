import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
export function FileInputs({ definitions, onPatch, }) {
    if (definitions.length === 0) {
        return null;
    }
    return (_jsxs("section", { className: "rounded-md border border-zinc-200 bg-white p-4", children: [_jsx("h2", { className: "text-sm font-semibold text-zinc-950", children: "Files" }), _jsx("div", { className: "mt-3 grid gap-3", children: definitions.map((definition) => (_jsxs("label", { className: "grid gap-1 text-sm font-medium text-zinc-700", children: [definition.label, _jsx("input", { className: "block w-full text-sm text-zinc-600 file:mr-3 file:rounded-md file:border-0 file:bg-zinc-950 file:px-3 file:py-2 file:text-sm file:font-semibold file:text-white", type: "file", accept: definition.accept, onChange: (event) => {
                                const file = event.target.files?.[0];
                                if (file) {
                                    void readFile(file, definition.encoding ?? "data-url").then((value) => onPatch(definition.targetPath, value));
                                }
                            } })] }, definition.id))) })] }));
}
function readFile(file, encoding) {
    return new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.addEventListener("load", () => resolve(String(reader.result ?? "")));
        reader.addEventListener("error", () => reject(reader.error ?? new Error("Unable to read file")));
        if (encoding === "text") {
            reader.readAsText(file);
        }
        else {
            reader.readAsDataURL(file);
        }
    });
}
//# sourceMappingURL=FileInputs.js.map