use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "geo-data-server",
    version,
    about = "Thin HTTP API adapter for geo-data"
)]
struct Args {
    /// Address to bind, for example 127.0.0.1:3000.
    #[arg(long, default_value = "127.0.0.1:3000")]
    addr: String,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    eprintln!("geo-data-server listening on http://{}", args.addr);
    geo_data_server::serve(&args.addr)
}
