use clap::{Parser, Subcommand};
use vector_analysis_index as _;

#[derive(Debug, Parser)]
#[command(
    name = "vector-analysis-index-cli",
    version,
    about = "Thin CLI adapter for vector-analysis-index"
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
            "vector-analysis-index",
            &vector_analysis_index_cli::package_metadata_json(),
        ),
        Command::Schema { json } => print_payload(
            json,
            "vector-analysis-index command schema",
            &vector_analysis_index_cli::command_schema_json(),
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
