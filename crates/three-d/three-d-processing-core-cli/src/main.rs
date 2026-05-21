use clap::{Parser, Subcommand};
use three_d_processing_core as _;

#[derive(Debug, Parser)]
#[command(
    name = "three-d-processing-core-cli",
    version,
    about = "Thin CLI adapter for three-d-processing-core"
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
            "three-d-processing-core",
            &three_d_processing_core_cli::package_metadata_json(),
        ),
        Command::Schema { json } => print_payload(
            json,
            "three-d-processing-core command schema",
            &three_d_processing_core_cli::command_schema_json(),
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
