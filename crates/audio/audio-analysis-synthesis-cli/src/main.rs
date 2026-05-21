use audio_analysis_synthesis as _;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "audio-analysis-synthesis-cli",
    version,
    about = "Thin CLI adapter for audio-analysis-synthesis"
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
            "audio-analysis-synthesis",
            &audio_analysis_synthesis_cli::package_metadata_json(),
        ),
        Command::Schema { json } => print_payload(
            json,
            "audio-analysis-synthesis command schema",
            &audio_analysis_synthesis_cli::command_schema_json(),
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
