use clap::{Parser, Subcommand};
use comfyui_data as _;

#[derive(Debug, Parser)]
#[command(
    name = "comfyui-data-cli",
    version,
    about = "Thin CLI adapter for comfyui-data"
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
            "comfyui-data",
            &comfyui_data_cli::package_metadata_json(),
        ),
        Command::Schema { json } => print_payload(
            json,
            "comfyui-data command schema",
            &comfyui_data_cli::command_schema_json(),
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
