use clap::{Parser, Subcommand};
use math_sparse_data as _;

#[derive(Debug, Parser)]
#[command(
    name = "math-sparse-data-cli",
    version,
    about = "Thin CLI adapter for math-sparse-data"
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
            "math-sparse-data",
            &math_sparse_data_cli::package_metadata_json(),
        ),
        Command::Schema { json } => print_payload(
            json,
            "math-sparse-data command schema",
            &math_sparse_data_cli::command_schema_json(),
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
