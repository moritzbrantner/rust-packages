use clap::{Parser, Subcommand};
use video_analysis_synthesis as _;

#[derive(Debug, Parser)]
#[command(
    name = "video-analysis-synthesis-cli",
    version,
    about = "Thin CLI adapter for video-analysis-synthesis"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print package and adapter metadata.
    Info {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Print the generic command schema.
    Schema {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Info { json: false }) {
        Command::Info { json } => print_payload(
            json,
            "video-analysis-synthesis",
            &video_analysis_synthesis_cli::package_metadata_json(),
        ),
        Command::Schema { json } => print_payload(
            json,
            "video-analysis-synthesis command schema",
            &video_analysis_synthesis_cli::command_schema_json(),
        ),
    }
}

fn print_payload(json: bool, title: &str, payload: &str) {
    if json {
        println!("{payload}");
    } else {
        println!("{title}");
        println!("{payload}");
    }
}
