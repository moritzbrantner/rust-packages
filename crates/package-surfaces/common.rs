#![allow(dead_code)]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

pub struct PackageMetadata {
    pub name: &'static str,
    pub version: &'static str,
    pub description: &'static str,
}

impl PackageMetadata {
    pub fn current() -> Self {
        Self {
            name: env!("CARGO_PKG_NAME"),
            version: env!("CARGO_PKG_VERSION"),
            description: env!("CARGO_PKG_DESCRIPTION"),
        }
    }

    pub fn json(&self) -> String {
        format!(
            "{{\"name\":\"{}\",\"version\":\"{}\",\"description\":\"{}\",\"surfaces\":[\"library\",\"cli\",\"api\",\"ui\"]}}",
            json_escape(self.name),
            json_escape(self.version),
            json_escape(self.description)
        )
    }

    pub fn html(&self) -> String {
        format!(
            "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{}</title><style>body{{font-family:system-ui,sans-serif;margin:0;background:#f8fafc;color:#18181b}}main{{max-width:760px;margin:0 auto;padding:48px 24px}}h1{{font-size:32px;margin:0 0 8px}}p{{font-size:16px;line-height:1.5;color:#52525b}}ul{{display:grid;grid-template-columns:repeat(auto-fit,minmax(140px,1fr));gap:12px;padding:0;list-style:none}}li{{border:1px solid #d4d4d8;background:white;border-radius:8px;padding:12px;font-weight:650}}</style></head><body><main><h1>{}</h1><p>{}</p><ul><li>Library</li><li>CLI Tool</li><li>API</li><li>Web UI</li></ul></main></body></html>",
            html_escape(self.name),
            html_escape(self.name),
            html_escape(self.description)
        )
    }
}

pub fn run_server(render_index: fn(&PackageMetadata) -> HttpResponse) -> std::io::Result<()> {
    let bind = parse_bind_address();
    let listener = TcpListener::bind(&bind)?;
    let address = listener.local_addr()?;
    println!(
        "{} listening on http://{}",
        PackageMetadata::current().name,
        address
    );

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let _ = handle_stream(&mut stream, render_index);
            }
            Err(error) => eprintln!("connection failed: {error}"),
        }
    }
    Ok(())
}

pub struct HttpResponse {
    pub status: &'static str,
    pub content_type: &'static str,
    pub body: String,
}

pub fn package_json_response(metadata: &PackageMetadata) -> HttpResponse {
    HttpResponse {
        status: "200 OK",
        content_type: "application/json; charset=utf-8",
        body: metadata.json(),
    }
}

pub fn package_html_response(metadata: &PackageMetadata) -> HttpResponse {
    HttpResponse {
        status: "200 OK",
        content_type: "text/html; charset=utf-8",
        body: metadata.html(),
    }
}

fn handle_stream(
    stream: &mut TcpStream,
    render_index: fn(&PackageMetadata) -> HttpResponse,
) -> std::io::Result<()> {
    let mut buffer = [0_u8; 2048];
    let read = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..read]);
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");
    let metadata = PackageMetadata::current();
    let response = match path {
        "/" => render_index(&metadata),
        "/health" => HttpResponse {
            status: "200 OK",
            content_type: "text/plain; charset=utf-8",
            body: "ok\n".to_string(),
        },
        "/api/package" => package_json_response(&metadata),
        _ => HttpResponse {
            status: "404 Not Found",
            content_type: "text/plain; charset=utf-8",
            body: "not found\n".to_string(),
        },
    };
    write_response(stream, response)
}

fn write_response(stream: &mut TcpStream, response: HttpResponse) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {}\r\ncontent-type: {}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        response.status,
        response.content_type,
        response.body.len(),
        response.body
    )
}

fn parse_bind_address() -> String {
    let mut host = "127.0.0.1".to_string();
    let mut port = "0".to_string();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--host" => {
                if let Some(value) = args.next() {
                    host = value;
                }
            }
            "--port" => {
                if let Some(value) = args.next() {
                    port = value;
                }
            }
            _ => {}
        }
    }
    format!("{host}:{port}")
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
