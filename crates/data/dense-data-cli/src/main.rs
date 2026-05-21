use clap::{Parser, Subcommand};
use dense_data as _;

#[derive(Debug, Parser)]
#[command(
    name = "dense-data-cli",
    version,
    about = "Thin CLI adapter for dense-data"
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
        Command::Info { json } => {
            print_payload(json, "dense-data", &dense_data_cli::package_metadata_json())
        }
        Command::Schema { json } => print_payload(
            json,
            "dense-data command schema",
            &dense_data_cli::command_schema_json(),
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
