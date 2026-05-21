use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "finance-statistics-server",
    version,
    about = "Thin HTTP API adapter for finance-statistics"
)]
struct Args {
    /// Address to bind, for example 127.0.0.1:3000.
    #[arg(long, default_value = "127.0.0.1:3000")]
    addr: String,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    eprintln!(
        "finance-statistics-server listening on http://{}",
        args.addr
    );
    finance_statistics_server::serve(&args.addr)
}
