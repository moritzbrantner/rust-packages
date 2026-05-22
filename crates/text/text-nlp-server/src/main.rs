use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "text-nlp-server",
    version,
    about = "HTTP API adapter for text-nlp-tasks"
)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:3000")]
    addr: String,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    eprintln!("text-nlp-server listening on http://{}", args.addr);
    text_nlp_server::serve(&args.addr)
}
