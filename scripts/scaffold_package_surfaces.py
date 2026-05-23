#!/usr/bin/env python3
"""Generate library-owned CLI, server, WASM, and React package surfaces."""

from __future__ import annotations

import json
import shutil
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
WRAPPER_SUFFIXES = ("-cli", "-server", "-wasm")


def main() -> None:
    metadata = cargo_metadata()
    workspace_members = set(metadata["workspace_members"])
    packages = [
        pkg
        for pkg in metadata["packages"]
        if pkg["id"] in workspace_members and is_library_crate(pkg)
    ]
    packages.sort(key=lambda pkg: pkg["name"])

    for package in packages:
        manifest = Path(package["manifest_path"])
        crate_dir = manifest.parent
        name = package["name"]
        description = package.get("description") or f"Runtime surface for the {name} library crate."

        if name != "runtime-contracts":
            add_dependency(manifest, "runtime-contracts.workspace = true")
        add_dependency(manifest, "serde_json.workspace = true")
        write_surface_module(crate_dir, name, description)
        expose_surface_module(crate_dir / "src" / "lib.rs")

        write_cli(crate_dir, name)
        write_server(crate_dir, name)
        write_wasm_crate(name)
        write_wasm_package(name)
        write_app(name, description)

    rewrite_root_package_scripts()
    print(f"generated CLI, server, WASM, and app surfaces for {len(packages)} library crates")


def cargo_metadata() -> dict:
    output = subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        cwd=ROOT,
        text=True,
    )
    return json.loads(output)


def is_library_crate(package: dict) -> bool:
    manifest_path = Path(package["manifest_path"])
    try:
        relative = manifest_path.relative_to(ROOT)
    except ValueError:
        return False

    parts = relative.parts
    if len(parts) < 3 or parts[0] != "crates":
        return False
    if parts[1] == "bindings":
        return False
    if package["name"].endswith(WRAPPER_SUFFIXES):
        return False
    return any("lib" in target["kind"] for target in package["targets"])


def add_dependency(manifest: Path, line: str) -> None:
    text = manifest.read_text()
    key = line.split(".", 1)[0].split("=", 1)[0].strip()
    deps_start = text.find("[dependencies]\n")
    if deps_start == -1:
        text += "\n[dependencies]\n"
        deps_start = text.find("[dependencies]\n")
    deps_end = text.find("\n[", deps_start + len("[dependencies]\n"))
    if deps_end == -1:
        deps_end = len(text)
    deps_section = text[deps_start:deps_end]
    if any(
        dep_line.startswith(f"{key}.") or dep_line.startswith(f"{key} ")
        or dep_line.startswith(f"{key}=")
        for dep_line in deps_section.splitlines()
    ):
        return
    text = text.replace("[dependencies]\n", f"[dependencies]\n{line}\n", 1)
    manifest.write_text(text)


def expose_surface_module(lib_rs: Path) -> None:
    text = lib_rs.read_text()
    text = text.replace("pub mod surface;\n", "")

    lines = text.splitlines()
    insert_at = 0
    while insert_at < len(lines) and (
        lines[insert_at].startswith("#![") or lines[insert_at].strip() == ""
        or lines[insert_at].startswith("//!")
    ):
        insert_at += 1
    lines.insert(insert_at, "pub mod surface;")
    lib_rs.write_text("\n".join(lines) + "\n")


def write_surface_module(crate_dir: Path, name: str, description: str) -> None:
    contract_import = "crate" if name == "runtime-contracts" else "runtime_contracts"
    write(
        crate_dir / "src" / "surface.rs",
        f"""//! Library-owned runtime surface for `{name}`.

use {contract_import}::{{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
}};

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {{
    PackageSurface {{
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![SurfaceOperation {{
            id: OperationId::new("describe"),
            name: "Describe package".to_string(),
            description: Some({json.dumps(description)}.to_string()),
            input_schema: serde_json::json!({{
                "type": "object",
                "additionalProperties": true
            }}),
            output_schema: serde_json::json!({{
                "type": "object",
                "required": ["library", "version", "operationCount"]
            }}),
            example_request: serde_json::json!({{
                "includeOperations": true
            }}),
            wasm_supported: true,
            server_supported: true,
        }}],
    }}
}}

/// Runs one library-owned operation.
pub fn run_surface_operation(
    request: SurfaceRequest,
) -> Result<SurfaceResponse, String> {{
    match request.operation.as_str() {{
        "describe" => {{
            let surface = package_surface();
            Ok(SurfaceResponse {{
                operation: request.operation,
                value: serde_json::json!({{
                    "library": surface.library,
                    "version": surface.version,
                    "operationCount": surface.operations.len(),
                    "operations": surface
                        .operations
                        .iter()
                        .map(|operation| operation.id.as_str())
                        .collect::<Vec<_>>(),
                    "input": request.input
                }}),
                diagnostics: Vec::new(),
                artifacts: Vec::new(),
            }})
        }}
        operation => Err(format!(
            "unsupported operation `{{operation}}` for {{}}",
            env!("CARGO_PKG_NAME")
        )),
    }}
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn package_surface_has_describe_operation() {{
        let surface = package_surface();
        assert_eq!(surface.library, env!("CARGO_PKG_NAME"));
        assert!(!surface.operations.is_empty());
    }}

    #[test]
    fn describe_operation_returns_surface_summary() {{
        let response = run_surface_operation(SurfaceRequest {{
            operation: OperationId::new("describe"),
            input: serde_json::json!({{"includeOperations": true}}),
        }})
        .expect("describe operation");

        assert_eq!(response.operation.as_str(), "describe");
        assert_eq!(response.value["library"], env!("CARGO_PKG_NAME"));
    }}
}}
""",
    )


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
{"" if name == "runtime-contracts" else "runtime-contracts.workspace = true"}
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
    let surface = {rust_ident(package_name)}::package_surface();
    assert_eq!(surface.library, "{name}");
    assert!(!surface.operations.is_empty());
}}
""",
    )
    write(
        wrapper_dir / "README.md",
        f"""# {package_name}

Thin command-line adapter for `{name}`.

Run:

```bash
cargo run -p {package_name} -- operations --json
cargo run -p {package_name} -- run --operation describe --json '{{"includeOperations":true}}'
```
""",
    )


def cli_lib_source(name: str) -> str:
    return f"""use runtime_contracts::{{OperationId, PackageSurface, SurfaceRequest, SurfaceResponse}};

/// Wrapped library crate name.
pub const LIBRARY_CRATE: &str = "{name}";
/// Adapter surface kind.
pub const SURFACE_KIND: &str = "cli";
/// Rust import path for the wrapped crate.
pub const LIBRARY_IMPORT: &str = "use {rust_ident(name)}";
/// Companion server package name.
pub const SERVER_PACKAGE: &str = "{name}-server";
/// Companion React app package name.
pub const APP_PACKAGE: &str = "{name}-app";
/// Companion WASM package name.
pub const WASM_PACKAGE: &str = "{name}-wasm";

pub fn package_surface() -> PackageSurface {{
    {rust_ident(name)}::surface::package_surface()
}}

pub fn package_metadata_json() -> String {{
    serde_json::json!({{
        "package": format!("{{}}-cli", LIBRARY_CRATE),
        "surface": SURFACE_KIND,
        "library": LIBRARY_CRATE,
        "libraryImport": LIBRARY_IMPORT,
        "serverPackage": SERVER_PACKAGE,
        "appPackage": APP_PACKAGE,
        "wasmPackage": WASM_PACKAGE,
        "operations": package_surface().operations
    }})
    .to_string()
}}

pub fn command_schema_json() -> String {{
    serde_json::json!({{
        "commands": [
            {{"name": "info", "description": "Print package and adapter metadata."}},
            {{"name": "schema", "description": "Print the CLI command schema."}},
            {{"name": "operations", "description": "Print library operations."}},
            {{"name": "run", "description": "Run one library-owned operation."}}
        ]
    }})
    .to_string()
}}

pub fn run_operation(operation: &str, input: serde_json::Value) -> Result<SurfaceResponse, String> {{
    {rust_ident(name)}::surface::run_surface_operation(SurfaceRequest {{
        operation: OperationId::new(operation),
        input,
    }})
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
    ident = rust_ident(package_name)
    return f"""use std::fs;
use std::io::Read;

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
    /// Print the command schema.
    Schema {{
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    }},
    /// Print library operations.
    Operations {{
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    }},
    /// Run one library-owned operation.
    Run {{
        /// Operation id.
        #[arg(long, default_value = "describe")]
        operation: String,
        /// JSON request payload.
        #[arg(long)]
        json: Option<String>,
        /// Read JSON request payload from a file.
        #[arg(long)]
        file: Option<String>,
    }},
}}

fn main() -> Result<(), Box<dyn std::error::Error>> {{
    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Info {{ json: false }}) {{
        Command::Info {{ json }} => print_payload(json, "{name}", &{ident}::package_metadata_json()),
        Command::Schema {{ json }} => print_payload(json, "{name} command schema", &{ident}::command_schema_json()),
        Command::Operations {{ json }} => {{
            let payload = serde_json::to_string(&{ident}::package_surface().operations)?;
            print_payload(json, "{name} operations", &payload);
        }}
        Command::Run {{ operation, json, file }} => {{
            let input = read_input(json, file)?;
            let response = {ident}::run_operation(&operation, input).map_err(std::io::Error::other)?;
            println!("{{}}", serde_json::to_string(&response)?);
        }}
    }}
    Ok(())
}}

fn read_input(json: Option<String>, file: Option<String>) -> Result<serde_json::Value, Box<dyn std::error::Error>> {{
    let input = if let Some(json) = json {{
        json
    }} else if let Some(file) = file {{
        fs::read_to_string(file)?
    }} else {{
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer)?;
        if buffer.trim().is_empty() {{
            "{{}}".to_string()
        }} else {{
            buffer
        }}
    }};
    Ok(serde_json::from_str(&input)?)
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
{"" if name == "runtime-contracts" else "runtime-contracts.workspace = true"}
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

#[test]
fn run_endpoint_calls_library_surface() {{
    let response = {rust_ident(package_name)}::response_for(
        "POST",
        "/api/run",
        r#"{{"operation":"describe","input":{{"includeOperations":true}}}}"#,
    );
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains(r#""operation""#));
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
- `GET /api/operations`
- `POST /api/run`
- `POST /api/<operation-id>`
""",
    )


def server_lib_source(name: str) -> str:
    return f"""use std::io::{{self, BufRead, BufReader, Read, Write}};
use std::net::{{TcpListener, TcpStream}};

use runtime_contracts::{{Diagnostic, DiagnosticSeverity, OperationId, SurfaceRequest}};

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
/// Companion WASM package name.
pub const WASM_PACKAGE: &str = "{name}-wasm";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {{
    pub status_code: u16,
    pub reason: &'static str,
    pub content_type: &'static str,
    pub body: String,
}}

pub fn serve(addr: &str) -> io::Result<()> {{
    let listener = TcpListener::bind(addr)?;
    for stream in listener.incoming() {{
        handle_stream(stream?)?;
    }}
    Ok(())
}}

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
        ("GET", "/api/operations") => json_response(
            200,
            "OK",
            serde_json::json!({rust_ident(name)}::surface::package_surface().operations),
        ),
        ("POST", "/api/run") => run_response(body),
        ("POST", path) if path.starts_with("/api/") => {{
            let operation = path.trim_start_matches("/api/");
            run_request(SurfaceRequest {{
                operation: OperationId::new(operation),
                input: parse_json_or_empty(body),
            }})
        }}
        _ => json_response(404, "Not Found", serde_json::json!({{
            "error": "not found",
            "path": path
        }})),
    }}
}}

pub fn package_metadata_json() -> String {{
    package_metadata_value().to_string()
}}

fn package_metadata_value() -> serde_json::Value {{
    let surface = {rust_ident(name)}::surface::package_surface();
    serde_json::json!({{
        "package": format!("{{}}-server", LIBRARY_CRATE),
        "surface": SURFACE_KIND,
        "library": LIBRARY_CRATE,
        "libraryImport": LIBRARY_IMPORT,
        "cliPackage": CLI_PACKAGE,
        "appPackage": APP_PACKAGE,
        "wasmPackage": WASM_PACKAGE,
        "endpoints": [
            "GET /health",
            "GET /api/package",
            "GET /api/schema",
            "GET /api/operations",
            "POST /api/run",
            "POST /api/<operation-id>"
        ],
        "operations": surface.operations
    }})
}}

fn schema_value() -> serde_json::Value {{
    let operations = {rust_ident(name)}::surface::package_surface()
        .operations
        .into_iter()
        .map(|operation| {{
            let path = format!("/api/{{}}", operation.id.as_str());
            (path, serde_json::json!({{
                "post": {{
                    "summary": operation.name,
                    "description": operation.description,
                    "requestBody": operation.input_schema,
                    "responses": {{"200": operation.output_schema}}
                }}
            }}))
        }})
        .collect::<serde_json::Map<_, _>>();

    serde_json::json!({{
        "openapi": "3.1.0",
        "info": {{
            "title": format!("{{}} API", LIBRARY_CRATE),
            "version": env!("CARGO_PKG_VERSION")
        }},
        "paths": operations
    }})
}}

fn run_response(body: &str) -> HttpResponse {{
    let payload = match serde_json::from_str::<serde_json::Value>(body) {{
        Ok(value) => value,
        Err(error) => {{
            return diagnostic_response(400, "Bad Request", "invalid_request", &format!("invalid JSON: {{error}}"));
        }}
    }};
    let operation = payload
        .get("operation")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("describe")
        .to_string();
    let input = payload.get("input").cloned().unwrap_or_else(|| payload.clone());
    run_request(SurfaceRequest {{
        operation: OperationId::new(operation),
        input,
    }})
}}

fn run_request(request: SurfaceRequest) -> HttpResponse {{
    match {rust_ident(name)}::surface::run_surface_operation(request) {{
        Ok(response) => json_response(200, "OK", serde_json::json!(response)),
        Err(error) => diagnostic_response(400, "Bad Request", "operation_failed", &error),
    }}
}}

fn diagnostic_response(status_code: u16, reason: &'static str, code: &str, message: &str) -> HttpResponse {{
    json_response(
        status_code,
        reason,
        serde_json::json!({{
            "diagnostics": [Diagnostic {{
                severity: DiagnosticSeverity::Error,
                code: code.into(),
                message: message.to_string(),
                source: Some(format!("{{}}-server", LIBRARY_CRATE)),
                help: None,
            }}]
        }}),
    )
}}

fn parse_json_or_empty(body: &str) -> serde_json::Value {{
    if body.trim().is_empty() {{
        serde_json::json!({{}})
    }} else {{
        serde_json::from_str(body).unwrap_or_else(|_| serde_json::json!({{"raw": body}}))
    }}
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


def write_wasm_crate(name: str) -> None:
    package_name = f"{name}-wasm"
    crate_dir = ROOT / "crates" / "bindings" / package_name
    src_dir = crate_dir / "src"
    src_dir.mkdir(parents=True, exist_ok=True)
    write(
        crate_dir / "Cargo.toml",
        f"""[package]
name = "{package_name}"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true
description = "WASM bindings for {name}."
repository.workspace = true
homepage.workspace = true
documentation.workspace = true
keywords.workspace = true
categories.workspace = true

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
js-sys = "0.3.82"
runtime-contracts.workspace = true
serde.workspace = true
serde_json.workspace = true
serde-wasm-bindgen = "0.6.5"
wasm-bindgen = "0.2.105"
{"" if name == "runtime-contracts" else f"{name}.workspace = true"}
""",
    )
    write(
        src_dir / "lib.rs",
        f"""//! WASM bindings for `{name}`.

use runtime_contracts::SurfaceRequest;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = packageSurface)]
pub fn package_surface() -> Result<JsValue, JsValue> {{
    serde_wasm_bindgen::to_value(&{rust_ident(name)}::surface::package_surface()).map_err(into_js_error)
}}

#[wasm_bindgen(js_name = runOperation)]
pub fn run_operation(request: JsValue) -> Result<JsValue, JsValue> {{
    let request: SurfaceRequest = serde_wasm_bindgen::from_value(request).map_err(into_js_error)?;
    let response = {rust_ident(name)}::surface::run_surface_operation(request).map_err(into_js_error)?;
    serde_wasm_bindgen::to_value(&response).map_err(into_js_error)
}}

fn into_js_error(error: impl std::fmt::Display) -> JsValue {{
    js_sys::Error::new(&error.to_string()).into()
}}

#[cfg(test)]
mod tests {{
    #[test]
    fn wrapped_surface_has_operations() {{
        let surface = {rust_ident(name)}::surface::package_surface();
        assert_eq!(surface.library, "{name}");
        assert!(!surface.operations.is_empty());
    }}
}}
""",
    )


def write_wasm_package(name: str) -> None:
    package_name = f"{name}-wasm"
    package_dir = ROOT / "packages" / package_name
    scripts_dir = package_dir / "scripts"
    tests_dir = package_dir / "tests"
    scripts_dir.mkdir(parents=True, exist_ok=True)
    tests_dir.mkdir(parents=True, exist_ok=True)
    scoped = f"@mb-rust/{package_name}"
    write(
        package_dir / "package.json",
        json.dumps(
            {
                "name": scoped,
                "version": "0.1.0",
                "description": f"WASM package for {name}.",
                "license": "MIT OR Apache-2.0",
                "type": "module",
                "private": False,
                "files": ["index.d.ts", "index.js", "pkg", "README.md"],
                "main": "./index.js",
                "types": "./index.d.ts",
                "exports": {
                    ".": {
                        "types": "./index.d.ts",
                        "import": "./index.js",
                        "default": "./index.js",
                    }
                },
                "scripts": {
                    "build": "bash scripts/build-wasm.sh",
                    "test": "bun test tests",
                },
            },
            indent=2,
        )
        + "\n",
    )
    write(
        scripts_dir / "build-wasm.sh",
        f"""#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${{BASH_SOURCE[0]}}")/../../.." && pwd)"
wasm-pack build "$ROOT_DIR/crates/bindings/{package_name}" --target web --out-dir "$ROOT_DIR/packages/{package_name}/pkg"
""",
    )
    (scripts_dir / "build-wasm.sh").chmod(0o755)
    write(
        package_dir / "index.js",
        f"""let wasmModulePromise;

export async function init() {{
  const wasmEntry = "./pkg/{rust_ident(package_name)}.js";
  wasmModulePromise ??= import(/* @vite-ignore */ wasmEntry).then(async (module) => {{
    if (typeof module.default === "function") {{
      await module.default();
    }}
    return module;
  }});
  return wasmModulePromise;
}}

export async function packageSurface() {{
  const module = await init();
  return module.packageSurface();
}}

export async function runOperation(request) {{
  const module = await init();
  return module.runOperation(request);
}}
""",
    )
    write(
        package_dir / "index.d.ts",
        """export interface SurfaceRequest {
  operation: string;
  input: unknown;
}

export interface SurfaceOperation {
  id: string;
  name: string;
  description?: string;
  inputSchema: unknown;
  outputSchema: unknown;
  exampleRequest: unknown;
  wasmSupported: boolean;
  serverSupported: boolean;
}

export interface PackageSurface {
  library: string;
  version: string;
  operations: SurfaceOperation[];
  capabilities: unknown;
}

export interface SurfaceResponse {
  operation: string;
  value: unknown;
  diagnostics: unknown[];
  artifacts: unknown[];
}

export function init(): Promise<unknown>;
export function packageSurface(): Promise<PackageSurface>;
export function runOperation(request: SurfaceRequest): Promise<SurfaceResponse>;
""",
    )
    write(
        tests_dir / "package.test.ts",
        f"""import {{ expect, test }} from "bun:test";

test("{package_name} package exports stable entrypoints", async () => {{
  const entry = await import("../index.js");
  expect(typeof entry.init).toBe("function");
  expect(typeof entry.packageSurface).toBe("function");
  expect(typeof entry.runOperation).toBe("function");
}});
""",
    )
    write(
        package_dir / "README.md",
        f"""# {scoped}

WASM package for `{name}`.

```bash
bun run --cwd packages/{package_name} build
```
""",
    )


def write_app(name: str, description: str) -> None:
    package_name = f"{name}-app"
    app_dir = ROOT / "packages" / package_name
    src_dir = app_dir / "src"
    if src_dir.exists():
        shutil.rmtree(src_dir)
    src_dir.mkdir(parents=True, exist_ok=True)
    title = title_case(name)
    wasm_package = f"@mb-rust/{name}-wasm"
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
                    wasm_package: "workspace:*",
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
    write(app_dir / "index.html", app_index_html(title))
    write(app_dir / "tsconfig.json", app_tsconfig())
    write(app_dir / "vite.config.ts", app_vite_config(name))
    write(app_dir / "postcss.config.ts", app_postcss_config())
    write(app_dir / "tailwind.config.ts", app_tailwind_config())
    write(src_dir / "api.ts", app_api_source(name))
    write(src_dir / "App.tsx", app_component_source(name, title, description))
    write(src_dir / "vite-env.d.ts", '/// <reference types="vite/client" />\n')
    write(src_dir / "main.tsx", app_main_source())
    write(src_dir / "styles.css", app_styles_source())
    write(
        app_dir / "README.md",
        f"""# {package_name}

React, TypeScript, TailwindCSS, Bun, and oxfmt frontend for `{name}`.

Run the server:

```bash
cargo run -p {name}-server -- --addr 127.0.0.1:3000
```

Run the app:

```bash
bun run --cwd packages/{package_name} dev
```
""",
    )


def app_api_source(name: str) -> str:
    wasm_package = f"@mb-rust/{name}-wasm"
    return f"""import {{ init, packageSurface, runOperation as runWasmOperation }} from "{wasm_package}";

export type RuntimeMode = "client-wasm" | "server";

export interface SurfaceOperation {{
  id: string;
  name: string;
  description?: string;
  inputSchema: unknown;
  outputSchema: unknown;
  exampleRequest: unknown;
  wasmSupported: boolean;
  serverSupported: boolean;
}}

export interface PackageSurface {{
  library: string;
  version: string;
  operations: SurfaceOperation[];
  capabilities: unknown;
}}

export interface SurfaceResponse {{
  operation: string;
  value: unknown;
  diagnostics: unknown[];
  artifacts: unknown[];
}}

export interface HealthPayload {{
  ok: boolean;
  package: string;
  library: string;
}}

const configuredServerUrl = import.meta.env.VITE_SERVER_URL as string | undefined;

export const serverBaseUrl = configuredServerUrl ?? "http://127.0.0.1:3000";
export const wrappedLibrary = "{name}";

export async function initializeWasm(): Promise<PackageSurface> {{
  await init();
  return packageSurface();
}}

export async function fetchHealth(): Promise<HealthPayload> {{
  return fetchJson<HealthPayload>("/health");
}}

export async function fetchServerSurface(): Promise<PackageSurface> {{
  const metadata = await fetchJson<{{ operations: SurfaceOperation[]; library: string }}>("/api/package");
  return {{
    library: metadata.library,
    version: "0.1.0",
    operations: metadata.operations ?? [],
    capabilities: {{}},
  }};
}}

export async function runOperation(mode: RuntimeMode, operation: string, input: unknown): Promise<SurfaceResponse> {{
  if (mode === "client-wasm") {{
    return runWasmOperation({{ operation, input }});
  }}
  const response = await fetch(`${{serverBaseUrl}}/api/run`, {{
    method: "POST",
    headers: {{ "content-type": "application/json" }},
    body: JSON.stringify({{ operation, input }}),
  }});
  if (!response.ok) {{
    throw new Error(`Server returned ${{response.status}}`);
  }}
  return response.json() as Promise<SurfaceResponse>;
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
  fetchServerSurface,
  initializeWasm,
  runOperation,
  serverBaseUrl,
  wrappedLibrary,
  type HealthPayload,
  type PackageSurface,
  type RuntimeMode,
  type SurfaceOperation,
}} from "./api";

type LoadState = "loading" | "ready" | "error";
const packageDescription = {json.dumps(description)};

export function App() {{
  const [runtimeMode, setRuntimeMode] = useState<RuntimeMode>("client-wasm");
  const [wasmState, setWasmState] = useState<LoadState>("loading");
  const [serverState, setServerState] = useState<LoadState>("loading");
  const [health, setHealth] = useState<HealthPayload | null>(null);
  const [surface, setSurface] = useState<PackageSurface | null>(null);
  const [selectedOperation, setSelectedOperation] = useState("describe");
  const [input, setInput] = useState("{{}}");
  const [result, setResult] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {{
    initializeWasm()
      .then((nextSurface) => {{
        setSurface(nextSurface);
        setSelectedOperation(nextSurface.operations[0]?.id ?? "describe");
        setInput(JSON.stringify(nextSurface.operations[0]?.exampleRequest ?? {{}}, null, 2));
        setWasmState("ready");
      }})
      .catch((caught) => {{
        setError(caught instanceof Error ? caught.message : String(caught));
        setWasmState("error");
      }});

    Promise.all([fetchHealth(), fetchServerSurface()])
      .then(([nextHealth, serverSurface]) => {{
        setHealth(nextHealth);
        setSurface((current) => current ?? serverSurface);
        setServerState("ready");
      }})
      .catch(() => setServerState("error"));
  }}, []);

  const operation = useMemo(
    () => surface?.operations.find((candidate) => candidate.id === selectedOperation) ?? surface?.operations[0],
    [selectedOperation, surface?.operations],
  );

  function chooseOperation(nextOperation: string) {{
    setSelectedOperation(nextOperation);
    const metadata = surface?.operations.find((candidate) => candidate.id === nextOperation);
    setInput(JSON.stringify(metadata?.exampleRequest ?? {{}}, null, 2));
    setResult("");
    setError(null);
  }}

  async function submit(event: FormEvent<HTMLFormElement>) {{
    event.preventDefault();
    setError(null);
    setResult("");
    try {{
      const payload = JSON.parse(input || "{{}}");
      const response = await runOperation(runtimeMode, selectedOperation, payload);
      setResult(JSON.stringify(response, null, 2));
    }} catch (caught) {{
      setError(caught instanceof Error ? caught.message : "Operation failed");
    }}
  }}

  return (
    <main className="min-h-screen bg-zinc-50 text-zinc-950">
      <section className="border-b border-zinc-200 bg-white">
        <div className="mx-auto flex max-w-6xl flex-col gap-4 px-5 py-5 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <p className="text-xs font-semibold uppercase tracking-wide text-teal-700">Package surface app</p>
            <h1 className="mt-1 text-2xl font-semibold">{title}</h1>
            <p className="mt-2 max-w-3xl text-sm leading-6 text-zinc-600">{{packageDescription}}</p>
          </div>
          <div className="segmented-control" role="group" aria-label="Runtime mode">
            <ModeButton active={{runtimeMode === "client-wasm"}} onClick={{() => setRuntimeMode("client-wasm")}}>
              Client WASM
            </ModeButton>
            <ModeButton active={{runtimeMode === "server"}} onClick={{() => setRuntimeMode("server")}}>
              Server API
            </ModeButton>
          </div>
        </div>
      </section>

      <section className="mx-auto grid max-w-6xl gap-5 px-5 py-6 lg:grid-cols-[minmax(0,1fr)_360px]">
        <form className="panel" onSubmit={{submit}}>
          <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
            <label className="grid flex-1 gap-1 text-sm">
              <span className="text-xs font-semibold uppercase tracking-wide text-zinc-500">Operation</span>
              <select
                className="rounded-md border border-zinc-300 px-3 py-2"
                value={{selectedOperation}}
                onChange={{(event) => chooseOperation(event.target.value)}}
              >
                {{(surface?.operations ?? []).map((candidate) => (
                  <option key={{candidate.id}} value={{candidate.id}}>
                    {{candidate.name}}
                  </option>
                ))}}
              </select>
            </label>
            <button className="button-primary" type="submit">
              Run
            </button>
          </div>
          <p className="section-copy mt-3">{{operation?.description ?? `Run ${{wrappedLibrary}} operation.`}}</p>
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
            <h2 className="section-title">Runtime</h2>
            <dl className="detail-list">
              <StatusRow label="WASM" state={{wasmState}} />
              <StatusRow label="Server" state={{serverState}} />
              <div>
                <dt>Server URL</dt>
                <dd>{{serverBaseUrl}}</dd>
              </div>
              <div>
                <dt>Health</dt>
                <dd>{{health?.package ?? "Not loaded"}}</dd>
              </div>
            </dl>
          </section>

          <section className="panel">
            <h2 className="section-title">Surface</h2>
            <dl className="detail-list">
              <div>
                <dt>Library</dt>
                <dd>{{surface?.library ?? wrappedLibrary}}</dd>
              </div>
              <div>
                <dt>Operations</dt>
                <dd>{{surface?.operations.length ?? 0}}</dd>
              </div>
              <div>
                <dt>Selected</dt>
                <dd>{{selectedOperation}}</dd>
              </div>
            </dl>
          </section>

          <section className="panel">
            <h2 className="section-title">Support</h2>
            <ul className="endpoint-list">
              {{(surface?.operations ?? []).map((candidate: SurfaceOperation) => (
                <li key={{candidate.id}}>
                  {{candidate.id}} · WASM {{candidate.wasmSupported ? "yes" : "no"}} · server{" "}
                  {{candidate.serverSupported ? "yes" : "no"}}
                </li>
              ))}}
            </ul>
          </section>
        </aside>
      </section>
    </main>
  );
}}

function ModeButton(props: {{ active: boolean; children: string; onClick: () => void }}) {{
  return (
    <button className={{props.active ? "mode-button mode-button-active" : "mode-button"}} type="button" onClick={{props.onClick}}>
      {{props.children}}
    </button>
  );
}}

function StatusRow(props: {{ label: string; state: LoadState }}) {{
  return (
    <div>
      <dt>{{props.label}}</dt>
      <dd>{{props.state === "ready" ? "Ready" : props.state === "error" ? "Unavailable" : "Loading"}}</dd>
    </div>
  );
}}
"""


def app_index_html(title: str) -> str:
    return f"""<!doctype html>
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
"""


def app_tsconfig() -> str:
    return """{
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
"""


def app_vite_config(name: str) -> str:
    return f"""import react from "@vitejs/plugin-react";
import {{ defineConfig }} from "vite";

export default defineConfig({{
  optimizeDeps: {{
    exclude: ["@mb-rust/{name}-wasm"],
  }},
  plugins: [react()],
}});
"""


def app_postcss_config() -> str:
    return """export default {
  plugins: {
    tailwindcss: {},
    autoprefixer: {},
  },
};
"""


def app_tailwind_config() -> str:
    return """import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {},
  },
  plugins: [],
} satisfies Config;
"""


def app_main_source() -> str:
    return """import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "./App";
import "./styles.css";

createRoot(document.getElementById("root") as HTMLElement).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
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

  .segmented-control {
    @apply inline-grid grid-cols-2 overflow-hidden rounded-md border border-zinc-300 bg-white;
  }

  .mode-button {
    @apply px-3 py-2 text-sm font-medium text-zinc-700 transition hover:bg-zinc-100;
  }

  .mode-button-active {
    @apply bg-zinc-950 text-white hover:bg-zinc-900;
  }

  .section-title {
    @apply text-base font-semibold text-zinc-950;
  }

  .section-copy {
    @apply text-sm text-zinc-600;
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


def rewrite_root_package_scripts() -> None:
    path = ROOT / "package.json"
    data = json.loads(path.read_text())
    scripts = data.setdefault("scripts", {})
    scripts["maps-wasm:bench"] = "bun run --cwd packages/maps-kernels-core-wasm bench:browser"
    scripts["maps-wasm:test"] = "bun run --cwd packages/maps-kernels-core-wasm test"
    scripts["text-nlp-wasm:test"] = "bun run --cwd packages/text-nlp-tasks-wasm test"
    path.write_text(json.dumps(data, indent=2) + "\n")


def title_case(name: str) -> str:
    return " ".join(
        part.upper() if part in {"io", "mvs", "onnx", "ocr", "nlp"} else part.capitalize()
        for part in name.split("-")
    )


def rust_ident(name: str) -> str:
    return name.replace("-", "_")


def write(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content)


if __name__ == "__main__":
    main()
