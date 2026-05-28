import type { FileInputDefinition } from "./types";

export function FileInputs({
  definitions,
  onPatch,
}: {
  definitions: FileInputDefinition[];
  onPatch: (path: string[], value: unknown) => void;
}) {
  if (definitions.length === 0) {
    return null;
  }

  return (
    <section className="rounded-md border border-zinc-200 bg-white p-4">
      <h2 className="text-sm font-semibold text-zinc-950">Files</h2>
      <div className="mt-3 grid gap-3">
        {definitions.map((definition) => (
          <label key={definition.id} className="grid gap-1 text-sm font-medium text-zinc-700">
            {definition.label}
            <input
              className="block w-full text-sm text-zinc-600 file:mr-3 file:rounded-md file:border-0 file:bg-zinc-950 file:px-3 file:py-2 file:text-sm file:font-semibold file:text-white"
              type="file"
              accept={definition.accept}
              onChange={(event) => {
                const file = event.target.files?.[0];
                if (file) {
                  void readFile(file, definition.encoding ?? "data-url").then((value) =>
                    onPatch(definition.targetPath, value),
                  );
                }
              }}
            />
          </label>
        ))}
      </div>
    </section>
  );
}

function readFile(file: File, encoding: "data-url" | "text"): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.addEventListener("load", () => resolve(String(reader.result ?? "")));
    reader.addEventListener("error", () => reject(reader.error ?? new Error("Unable to read file")));
    if (encoding === "text") {
      reader.readAsText(file);
    } else {
      reader.readAsDataURL(file);
    }
  });
}

