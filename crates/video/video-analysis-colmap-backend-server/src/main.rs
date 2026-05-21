use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "video-analysis-colmap-backend-server",
    version,
    about = "Thin HTTP API adapter for video-analysis-colmap-backend"
)]
struct Args {
    /// Address to bind, for example 127.0.0.1:3000.
    #[arg(long, default_value = "127.0.0.1:3000")]
    addr: String,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    eprintln!(
        "video-analysis-colmap-backend-server listening on http://{}",
        args.addr
    );
    video_analysis_colmap_backend_server::serve(&args.addr)
}
