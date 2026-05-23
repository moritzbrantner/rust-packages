use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "runtime-jobs-server",
    version,
    about = "Thin HTTP API adapter for runtime-jobs"
)]
struct Args {
    /// Address to bind, for example 127.0.0.1:3000.
    #[arg(long, default_value = "127.0.0.1:3000")]
    addr: String,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    eprintln!("runtime-jobs-server listening on http://{}", args.addr);
    runtime_jobs_server::serve(&args.addr)
}
