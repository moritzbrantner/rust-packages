use clap::{Parser, Subcommand};
use video_analysis_transform as _;

#[derive(Debug, Parser)]
#[command(
    name = "video-analysis-transform-cli",
    version,
    about = "Thin CLI adapter for video-analysis-transform"
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
            "video-analysis-transform",
            &video_analysis_transform_cli::package_metadata_json(),
        ),
        Command::Schema { json } => print_payload(
            json,
            "video-analysis-transform command schema",
            &video_analysis_transform_cli::command_schema_json(),
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
