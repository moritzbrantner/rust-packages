use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "comfyui-latents-server",
    version,
    about = "Thin HTTP API adapter for comfyui-latents"
)]
struct Args {
    /// Address to bind, for example 127.0.0.1:3000.
    #[arg(long, default_value = "127.0.0.1:3000")]
    addr: String,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    eprintln!("comfyui-latents-server listening on http://{}", args.addr);
    comfyui_latents_server::serve(&args.addr)
}
