use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "video-analysis-test-support-server",
    version,
    about = "Thin HTTP API adapter for video-analysis-test-support"
)]
struct Args {
    /// Address to bind, for example 127.0.0.1:3000.
    #[arg(long, default_value = "127.0.0.1:3000")]
    addr: String,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    eprintln!(
        "video-analysis-test-support-server listening on http://{}",
        args.addr
    );
    video_analysis_test_support_server::serve(&args.addr)
}
