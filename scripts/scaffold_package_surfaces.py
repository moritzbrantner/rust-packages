#!/usr/bin/env python3
"""Generate thin CLI, HTTP server, and React app surfaces for library crates."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def main() -> None:
    metadata = cargo_metadata()
    workspace_members = set(metadata["workspace_members"])
    packages = [
        pkg
        for pkg in metadata["packages"]
        if pkg["id"] in workspace_members and is_wrappable_library(pkg)
    ]
    packages.sort(key=lambda pkg: pkg["name"])

    for package in packages:
        manifest = Path(package["manifest_path"])
        crate_dir = manifest.parent
        name = package["name"]
        description = package.get("description") or f"Companion frontend for the {name} library crate."
        write_cli(crate_dir, name)
        write_server(crate_dir, name)
        write_app(name, description)

    rewrite_workspace_members()
    print(f"generated surfaces for {len(packages)} library crates")


def cargo_metadata() -> dict:
    output = subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        cwd=ROOT,
        text=True,
    )
    return json.loads(output)


def is_wrappable_library(package: dict) -> bool:
    manifest_path = Path(package["manifest_path"])
    try:
        relative = manifest_path.relative_to(ROOT)
    except ValueError:
        return False

    parts = relative.parts
    if len(parts) < 3 or parts[0] != "crates":
        return False
    if parts[1] in {"bindings", "test-support"}:
        return False

    name = package["name"]
    if name.endswith(("-cli", "-server", "-app", "-test-support")):
        return False

    return any("lib" in target["kind"] for target in package["targets"])


def write_cli(crate_dir: Path, name: str) -> None:
    package_name = f"{name}-cli"
    wrapper_dir = crate_dir.parent / package_name
    src_dir = wrapper_dir / "src"
    tests_dir = wrapper_dir / "tests"
    src_dir.mkdir(parents=True, exist_ok=True)
    tests_dir.mkdir(parents=True, exist_ok=True)

    write(
        wrapper_dir / "Cargo.toml",
        f"""[package]
name = "{package_name}"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true
description = "Command-line adapter for the {name} library crate."
repository.workspace = true
homepage.workspace = true
documentation.workspace = true
keywords.workspace = true
categories.workspace = true
default-run = "{package_name}"

[[bin]]
name = "{package_name}"
path = "src/main.rs"

[dependencies]
clap.workspace = true
serde_json.workspace = true
{name} = {{ path = "../{name}" }}
""",
    )
    write(src_dir / "lib.rs", cli_lib_source(name))
    write(src_dir / "main.rs", cli_main_source(name))
    write(
        tests_dir / "cli_surface.rs",
        f"""#[test]
fn cli_adapter_reports_wrapped_library() {{
    assert_eq!({rust_ident(package_name)}::LIBRARY_CRATE, "{name}");
    assert_eq!({rust_ident(package_name)}::SURFACE_KIND, "cli");
}}
""",
    )
    write(
        wrapper_dir / "README.md",
        f"""# {package_name}

Thin command-line adapter for `{name}`.

Run:

```bash
cargo run -p {package_name} -- info --json
```
""",
    )


def cli_lib_source(name: str) -> str:
    return f"""/// Wrapped library crate name.
pub const LIBRARY_CRATE: &str = "{name}";
/// Adapter surface kind.
pub const SURFACE_KIND: &str = "cli";
/// Rust import path for the wrapped crate.
pub const LIBRARY_IMPORT: &str = "use {rust_ident(name)}";
/// Companion server package name.
pub const SERVER_PACKAGE: &str = "{name}-server";
/// Companion React app package name.
pub const APP_PACKAGE: &str = "{name}-app";

/// Returns JSON metadata for this CLI adapter.
pub fn package_metadata_json() -> String {{
    serde_json::json!({{
        "package": format!("{{}}-cli", LIBRARY_CRATE),
        "surface": SURFACE_KIND,
        "library": LIBRARY_CRATE,
        "libraryImport": LIBRARY_IMPORT,
        "serverPackage": SERVER_PACKAGE,
        "appPackage": APP_PACKAGE
    }})
    .to_string()
}}

/// Returns a compact command schema for this generic CLI adapter.
pub fn command_schema_json() -> String {{
    serde_json::json!({{
        "commands": [
            {{
                "name": "info",
                "description": "Print package and adapter metadata."
            }},
            {{
                "name": "schema",
                "description": "Print the generic CLI command schema."
            }}
        ]
    }})
    .to_string()
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn metadata_mentions_wrapped_library() {{
        let metadata = package_metadata_json();
        assert!(metadata.contains(LIBRARY_CRATE));
        assert!(metadata.contains(SURFACE_KIND));
    }}
}}
"""


def cli_main_source(name: str) -> str:
    package_name = f"{name}-cli"
    return f"""use {rust_ident(name)} as _;
use clap::{{Parser, Subcommand}};

#[derive(Debug, Parser)]
#[command(name = "{package_name}", version, about = "Thin CLI adapter for {name}")]
struct Cli {{
    #[command(subcommand)]
    command: Option<Command>,
}}

#[derive(Debug, Subcommand)]
enum Command {{
    /// Print package and adapter metadata.
    Info {{
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    }},
    /// Print the generic command schema.
    Schema {{
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    }},
}}

fn main() {{
    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Info {{ json: false }}) {{
        Command::Info {{ json }} => print_payload(
            json,
            "{name}",
            &{rust_ident(package_name)}::package_metadata_json(),
        ),
        Command::Schema {{ json }} => print_payload(
            json,
            "{name} command schema",
            &{rust_ident(package_name)}::command_schema_json(),
        ),
    }}
}}

fn print_payload(json: bool, title: &str, payload: &str) {{
    if json {{
        println!("{{payload}}");
    }} else {{
        println!("{{title}}");
        println!("{{payload}}");
    }}
}}
"""


def write_server(crate_dir: Path, name: str) -> None:
    package_name = f"{name}-server"
    wrapper_dir = crate_dir.parent / package_name
    src_dir = wrapper_dir / "src"
    tests_dir = wrapper_dir / "tests"
    src_dir.mkdir(parents=True, exist_ok=True)
    tests_dir.mkdir(parents=True, exist_ok=True)

    write(
        wrapper_dir / "Cargo.toml",
        f"""[package]
name = "{package_name}"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true
description = "HTTP API adapter for the {name} library crate."
repository.workspace = true
homepage.workspace = true
documentation.workspace = true
keywords.workspace = true
categories.workspace = true
default-run = "{package_name}"

[[bin]]
name = "{package_name}"
path = "src/main.rs"

[dependencies]
clap.workspace = true
serde_json.workspace = true
{name} = {{ path = "../{name}" }}
""",
    )
    write(src_dir / "lib.rs", server_lib_source(name))
    write(src_dir / "main.rs", server_main_source(name))
    write(
        tests_dir / "server_surface.rs",
        f"""#[test]
fn package_endpoint_reports_wrapped_library() {{
    let response = {rust_ident(package_name)}::response_for("GET", "/api/package", "");
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("{name}"));
}}
""",
    )
    write(
        wrapper_dir / "README.md",
        f"""# {package_name}

Thin HTTP API adapter for `{name}`.

Run:

```bash
cargo run -p {package_name} -- --addr 127.0.0.1:3000
```

Endpoints:

- `GET /health`
- `GET /api/package`
- `GET /api/schema`
- `POST /api/run`
""",
    )


def server_lib_source(name: str) -> str:
    return f"""use {rust_ident(name)} as _;
use std::io::{{self, BufRead, BufReader, Read, Write}};
use std::net::{{TcpListener, TcpStream}};

/// Wrapped library crate name.
pub const LIBRARY_CRATE: &str = "{name}";
/// Adapter surface kind.
pub const SURFACE_KIND: &str = "api";
/// Rust import path for the wrapped crate.
pub const LIBRARY_IMPORT: &str = "use {rust_ident(name)}";
/// Companion CLI package name.
pub const CLI_PACKAGE: &str = "{name}-cli";
/// Companion React app package name.
pub const APP_PACKAGE: &str = "{name}-app";

/// Minimal HTTP response used by the generated API adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {{
    pub status_code: u16,
    pub reason: &'static str,
    pub content_type: &'static str,
    pub body: String,
}}

/// Serves the package API on the provided socket address.
pub fn serve(addr: &str) -> io::Result<()> {{
    let listener = TcpListener::bind(addr)?;
    for stream in listener.incoming() {{
        handle_stream(stream?)?;
    }}
    Ok(())
}}

/// Returns a response for one generated API request.
pub fn response_for(method: &str, path: &str, body: &str) -> HttpResponse {{
    match (method, path) {{
        ("OPTIONS", _) => HttpResponse {{
            status_code: 204,
            reason: "No Content",
            content_type: "application/json",
            body: String::new(),
        }},
        ("GET", "/health") => json_response(200, "OK", serde_json::json!({{
            "ok": true,
            "package": format!("{{}}-server", LIBRARY_CRATE),
            "library": LIBRARY_CRATE
        }})),
        ("GET", "/api/package") => json_response(200, "OK", package_metadata_value()),
        ("GET", "/api/schema") => json_response(200, "OK", schema_value()),
        ("POST", "/api/run") => json_response(200, "OK", serde_json::json!({{
            "package": format!("{{}}-server", LIBRARY_CRATE),
            "library": LIBRARY_CRATE,
            "accepted": true,
            "input": body,
            "note": "This generic adapter is ready for crate-specific operations."
        }})),
        _ => json_response(404, "Not Found", serde_json::json!({{
            "error": "not found",
            "path": path
        }})),
    }}
}}

/// Returns JSON metadata for this server adapter.
pub fn package_metadata_json() -> String {{
    package_metadata_value().to_string()
}}

fn package_metadata_value() -> serde_json::Value {{
    serde_json::json!({{
        "package": format!("{{}}-server", LIBRARY_CRATE),
        "surface": SURFACE_KIND,
        "library": LIBRARY_CRATE,
        "libraryImport": LIBRARY_IMPORT,
        "cliPackage": CLI_PACKAGE,
        "appPackage": APP_PACKAGE,
        "endpoints": [
            "GET /health",
            "GET /api/package",
            "GET /api/schema",
            "POST /api/run"
        ]
    }})
}}

fn schema_value() -> serde_json::Value {{
    serde_json::json!({{
        "openapi": "3.1.0",
        "info": {{
            "title": format!("{{}} API", LIBRARY_CRATE),
            "version": env!("CARGO_PKG_VERSION")
        }},
        "paths": {{
            "/health": {{ "get": {{ "summary": "Health check" }} }},
            "/api/package": {{ "get": {{ "summary": "Package metadata" }} }},
            "/api/schema": {{ "get": {{ "summary": "API schema" }} }},
            "/api/run": {{ "post": {{ "summary": "Generic operation entrypoint" }} }}
        }}
    }})
}}

fn handle_stream(mut stream: TcpStream) -> io::Result<()> {{
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    let mut content_length = 0usize;
    loop {{
        let mut header = String::new();
        reader.read_line(&mut header)?;
        let trimmed = header.trim_end();
        if trimmed.is_empty() {{
            break;
        }}
        if let Some((name, value)) = trimmed.split_once(':') {{
            if name.eq_ignore_ascii_case("content-length") {{
                content_length = value.trim().parse().unwrap_or(0);
            }}
        }}
    }}

    let mut body = vec![0; content_length];
    if content_length > 0 {{
        reader.read_exact(&mut body)?;
    }}
    let body = String::from_utf8_lossy(&body);

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let path = parts.next().unwrap_or("/");
    let response = response_for(method, path, &body);
    write_response(&mut stream, response)
}}

fn json_response(status_code: u16, reason: &'static str, value: serde_json::Value) -> HttpResponse {{
    HttpResponse {{
        status_code,
        reason,
        content_type: "application/json",
        body: value.to_string(),
    }}
}}

fn write_response(stream: &mut TcpStream, response: HttpResponse) -> io::Result<()> {{
    write!(
        stream,
        "HTTP/1.1 {{}} {{}}\\r\\nContent-Type: {{}}\\r\\nContent-Length: {{}}\\r\\nAccess-Control-Allow-Origin: *\\r\\nAccess-Control-Allow-Headers: content-type\\r\\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\\r\\nConnection: close\\r\\n\\r\\n{{}}",
        response.status_code,
        response.reason,
        response.content_type,
        response.body.len(),
        response.body
    )
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn health_endpoint_reports_package() {{
        let response = response_for("GET", "/health", "");
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains(LIBRARY_CRATE));
    }}
}}
"""


def server_main_source(name: str) -> str:
    package_name = f"{name}-server"
    return f"""use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "{package_name}", version, about = "Thin HTTP API adapter for {name}")]
struct Args {{
    /// Address to bind, for example 127.0.0.1:3000.
    #[arg(long, default_value = "127.0.0.1:3000")]
    addr: String,
}}

fn main() -> std::io::Result<()> {{
    let args = Args::parse();
    eprintln!("{package_name} listening on http://{{}}", args.addr);
    {rust_ident(package_name)}::serve(&args.addr)
}}
"""


def write_app(name: str, description: str) -> None:
    package_name = f"{name}-app"
    app_dir = ROOT / "packages" / package_name
    src_dir = app_dir / "src"
    src_dir.mkdir(parents=True, exist_ok=True)

    title = title_case(name)
    write(
        app_dir / "package.json",
        json.dumps(
            {
                "name": package_name,
                "version": "0.1.0",
                "description": f"React app for {name}: {description}",
                "private": True,
                "type": "module",
                "packageManager": "bun@1.3.14",
                "scripts": {
                    "dev": "vite --host 0.0.0.0",
                    "build": "tsc -p tsconfig.json && vite build",
                    "typecheck": "tsc -p tsconfig.json --noEmit",
                    "format": "bunx oxfmt --write src",
                    "preview": "vite preview --host 0.0.0.0",
                },
                "dependencies": {
                    "@vitejs/plugin-react": "^5.1.2",
                    "react": "^19.2.5",
                    "react-dom": "^19.2.5",
                    "vite": "^7.3.0",
                },
                "devDependencies": {
                    "@types/react": "^19.2.14",
                    "@types/react-dom": "^19.2.3",
                    "autoprefixer": "^10.4.23",
                    "postcss": "^8.5.6",
                    "tailwindcss": "^3.4.18",
                    "typescript": "^6.0.2",
                },
            },
            indent=2,
        )
        + "\n",
    )
    write(
        app_dir / "index.html",
        f"""<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>{title}</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
""",
    )
    write(
        app_dir / "tsconfig.json",
        """{
  "compilerOptions": {
    "target": "ES2022",
    "useDefineForClassFields": true,
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "allowJs": false,
    "skipLibCheck": true,
    "esModuleInterop": true,
    "allowSyntheticDefaultImports": true,
    "strict": true,
    "forceConsistentCasingInFileNames": true,
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx"
  },
  "include": ["src"],
  "references": []
}
""",
    )
    write(
        app_dir / "vite.config.ts",
        """import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react()],
});
""",
    )
    write(
        app_dir / "postcss.config.ts",
        """export default {
  plugins: {
    tailwindcss: {},
    autoprefixer: {},
  },
};
""",
    )
    write(
        app_dir / "tailwind.config.ts",
        """import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {},
  },
  plugins: [],
} satisfies Config;
""",
    )
    write(src_dir / "api.ts", app_api_source(name))
    write(src_dir / "App.tsx", app_component_source(name, title, description))
    write(src_dir / "vite-env.d.ts", '/// <reference types="vite/client" />\n')
    write(
        src_dir / "main.tsx",
        """import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "./App";
import "./styles.css";

createRoot(document.getElementById("root") as HTMLElement).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
""",
    )
    write(src_dir / "styles.css", app_styles_source())
    write(
        app_dir / "README.md",
        f"""# {package_name}

React, TypeScript, TailwindCSS, Bun, and oxfmt frontend for `{name}-server`.

Run the server first:

```bash
cargo run -p {name}-server -- --addr 127.0.0.1:3000
```

Then run the app:

```bash
bun run --cwd packages/{package_name} dev
```
""",
    )


def app_api_source(name: str) -> str:
    return f"""export interface PackageMetadata {{
  package: string;
  surface: string;
  library: string;
  libraryImport: string;
  cliPackage: string;
  appPackage: string;
  endpoints: string[];
}}

export interface HealthPayload {{
  ok: boolean;
  package: string;
  library: string;
}}

const configuredServerUrl = import.meta.env.VITE_SERVER_URL as string | undefined;

export const serverBaseUrl = configuredServerUrl ?? "http://127.0.0.1:3000";
export const wrappedLibrary = "{name}";

export async function fetchHealth(): Promise<HealthPayload> {{
  return fetchJson<HealthPayload>("/health");
}}

export async function fetchPackageMetadata(): Promise<PackageMetadata> {{
  return fetchJson<PackageMetadata>("/api/package");
}}

export async function runOperation(input: string): Promise<unknown> {{
  const response = await fetch(`${{serverBaseUrl}}/api/run`, {{
    method: "POST",
    headers: {{ "content-type": "application/json" }},
    body: input,
  }});
  if (!response.ok) {{
    throw new Error(`Server returned ${{response.status}}`);
  }}
  return response.json() as Promise<unknown>;
}}

async function fetchJson<T>(path: string): Promise<T> {{
  const response = await fetch(`${{serverBaseUrl}}${{path}}`);
  if (!response.ok) {{
    throw new Error(`Server returned ${{response.status}}`);
  }}
  return response.json() as Promise<T>;
}}
"""


def app_component_source(name: str, title: str, description: str) -> str:
    return f"""import {{ FormEvent, useEffect, useMemo, useState }} from "react";

import {{
  fetchHealth,
  fetchPackageMetadata,
  runOperation,
  serverBaseUrl,
  wrappedLibrary,
  type HealthPayload,
  type PackageMetadata,
}} from "./api";

type LoadState = "idle" | "loading" | "ready" | "error";
const packageDescription = {json.dumps(description)};

export function App() {{
  const [loadState, setLoadState] = useState<LoadState>("idle");
  const [health, setHealth] = useState<HealthPayload | null>(null);
  const [metadata, setMetadata] = useState<PackageMetadata | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [input, setInput] = useState('{{"operation":"introspect"}}');
  const [result, setResult] = useState<string>("");

  useEffect(() => {{
    void refresh();
  }}, []);

  const statusLabel = useMemo(() => {{
    if (loadState === "ready" && health?.ok) {{
      return "Online";
    }}
    if (loadState === "error") {{
      return "Offline";
    }}
    return "Checking";
  }}, [health?.ok, loadState]);

  async function refresh() {{
    setLoadState("loading");
    setError(null);
    try {{
      const [nextHealth, nextMetadata] = await Promise.all([fetchHealth(), fetchPackageMetadata()]);
      setHealth(nextHealth);
      setMetadata(nextMetadata);
      setLoadState("ready");
    }} catch (caught) {{
      setError(caught instanceof Error ? caught.message : "Unable to reach the server");
      setLoadState("error");
    }}
  }}

  async function submit(event: FormEvent<HTMLFormElement>) {{
    event.preventDefault();
    setError(null);
    try {{
      const payload = await runOperation(input);
      setResult(JSON.stringify(payload, null, 2));
    }} catch (caught) {{
      setResult("");
      setError(caught instanceof Error ? caught.message : "Operation failed");
    }}
  }}

  return (
    <main className="min-h-screen bg-zinc-50 text-zinc-950">
      <section className="border-b border-zinc-200 bg-white">
        <div className="mx-auto flex max-w-6xl flex-col gap-4 px-5 py-5 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <p className="text-xs font-semibold uppercase tracking-wide text-teal-700">Package app</p>
            <h1 className="mt-1 text-2xl font-semibold">{title}</h1>
            <p className="mt-2 max-w-3xl text-sm leading-6 text-zinc-600">{{packageDescription}}</p>
          </div>
          <div className="flex items-center gap-3">
            <span className={{`status-pill ${{loadState === "ready" ? "status-online" : loadState === "error" ? "status-offline" : "status-pending"}}`}}>
              {{statusLabel}}
            </span>
            <button className="button-secondary" type="button" onClick={{refresh}}>
              Refresh
            </button>
          </div>
        </div>
      </section>

      <section className="mx-auto grid max-w-6xl gap-5 px-5 py-6 lg:grid-cols-[minmax(0,1fr)_360px]">
        <form className="panel" onSubmit={{submit}}>
          <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
            <div>
              <h2 className="section-title">API operation</h2>
              <p className="section-copy">POST payload for {name}-server.</p>
            </div>
            <button className="button-primary" type="submit">
              Run
            </button>
          </div>
          <textarea
            className="code-input mt-4"
            spellCheck={{false}}
            value={{input}}
            onChange={{(event) => setInput(event.target.value)}}
          />
          {{result ? <pre className="result-block">{{result}}</pre> : null}}
          {{error ? <p className="error-text">{{error}}</p> : null}}
        </form>

        <aside className="space-y-5">
          <section className="panel">
            <h2 className="section-title">Server</h2>
            <dl className="detail-list">
              <div>
                <dt>URL</dt>
                <dd>{{serverBaseUrl}}</dd>
              </div>
              <div>
                <dt>Health</dt>
                <dd>{{health?.package ?? "Not loaded"}}</dd>
              </div>
            </dl>
          </section>

          <section className="panel">
            <h2 className="section-title">Package</h2>
            <dl className="detail-list">
              <div>
                <dt>Library</dt>
                <dd>{{metadata?.library ?? wrappedLibrary}}</dd>
              </div>
              <div>
                <dt>Import</dt>
                <dd>{{metadata?.libraryImport ?? "Loading"}}</dd>
              </div>
              <div>
                <dt>CLI</dt>
                <dd>{{metadata?.cliPackage ?? `${{wrappedLibrary}}-cli`}}</dd>
              </div>
              <div>
                <dt>App</dt>
                <dd>{{metadata?.appPackage ?? `${{wrappedLibrary}}-app`}}</dd>
              </div>
            </dl>
          </section>

          <section className="panel">
            <h2 className="section-title">Endpoints</h2>
            <ul className="endpoint-list">
              {{(metadata?.endpoints ?? ["GET /health", "GET /api/package", "GET /api/schema", "POST /api/run"]).map(
                (endpoint) => (
                  <li key={{endpoint}}>{{endpoint}}</li>
                ),
              )}}
            </ul>
          </section>
        </aside>
      </section>
    </main>
  );
}}
"""


def app_styles_source() -> str:
    return """@tailwind base;
@tailwind components;
@tailwind utilities;

@layer components {
  .panel {
    @apply rounded-md border border-zinc-200 bg-white p-5 shadow-sm;
  }

  .button-primary {
    @apply rounded-md bg-teal-700 px-4 py-2 text-sm font-semibold text-white transition hover:bg-teal-800 focus:outline-none focus:ring-2 focus:ring-teal-600 focus:ring-offset-2;
  }

  .button-secondary {
    @apply rounded-md border border-zinc-300 bg-white px-3 py-2 text-sm font-medium text-zinc-800 transition hover:bg-zinc-100 focus:outline-none focus:ring-2 focus:ring-teal-600 focus:ring-offset-2;
  }

  .status-pill {
    @apply inline-flex min-w-20 items-center justify-center rounded-md px-3 py-1.5 text-xs font-semibold uppercase tracking-wide;
  }

  .status-online {
    @apply bg-emerald-100 text-emerald-800;
  }

  .status-offline {
    @apply bg-rose-100 text-rose-800;
  }

  .status-pending {
    @apply bg-amber-100 text-amber-800;
  }

  .section-title {
    @apply text-base font-semibold text-zinc-950;
  }

  .section-copy {
    @apply mt-1 text-sm text-zinc-600;
  }

  .code-input {
    @apply min-h-56 w-full resize-y rounded-md border border-zinc-300 bg-zinc-950 p-4 font-mono text-sm text-zinc-50 outline-none focus:border-teal-500 focus:ring-2 focus:ring-teal-200;
  }

  .result-block {
    @apply mt-4 max-h-80 overflow-auto rounded-md border border-zinc-200 bg-zinc-100 p-4 font-mono text-sm text-zinc-900;
  }

  .error-text {
    @apply mt-4 rounded-md border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-800;
  }

  .detail-list {
    @apply mt-4 space-y-3 text-sm;
  }

  .detail-list div {
    @apply grid gap-1;
  }

  .detail-list dt {
    @apply text-xs font-semibold uppercase tracking-wide text-zinc-500;
  }

  .detail-list dd {
    @apply break-words font-mono text-zinc-900;
  }

  .endpoint-list {
    @apply mt-4 space-y-2 font-mono text-sm text-zinc-800;
  }
}
"""


def rewrite_workspace_members() -> None:
    manifest = ROOT / "Cargo.toml"
    text = manifest.read_text()
    start = text.index("[workspace]\n")
    members_start = text.index("members = [", start)
    members_end = text.index("]\n", members_start) + 2
    replacement = """members = [
    ".",
    "crates/*/*",
    "prototypes/rust/video-analysis-use-cases",
]
"""
    manifest.write_text(text[:members_start] + replacement + text[members_end:])


def title_case(name: str) -> str:
    return " ".join(part.upper() if part in {"io", "mvs", "onnx"} else part.capitalize() for part in name.split("-"))


def rust_ident(name: str) -> str:
    return name.replace("-", "_")


def write(path: Path, content: str) -> None:
    path.write_text(content)


if __name__ == "__main__":
    main()
