use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "three-d-scene-svg-server",
    version,
    about = "Thin HTTP API adapter for three-d-scene-svg"
)]
struct Args {
    /// Address to bind, for example 127.0.0.1:3000.
    #[arg(long, default_value = "127.0.0.1:3000")]
    addr: String,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    eprintln!("three-d-scene-svg-server listening on http://{}", args.addr);
    three_d_scene_svg_server::serve(&args.addr)
}
