use clap::{Parser, Subcommand};
use math_statistics as _;

#[derive(Debug, Parser)]
#[command(
    name = "math-statistics-cli",
    version,
    about = "Thin CLI adapter for math-statistics"
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
            "math-statistics",
            &math_statistics_cli::package_metadata_json(),
        ),
        Command::Schema { json } => print_payload(
            json,
            "math-statistics command schema",
            &math_statistics_cli::command_schema_json(),
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
