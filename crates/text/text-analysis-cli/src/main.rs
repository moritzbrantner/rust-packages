use std::fs;
use std::io::Read;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "text-analysis-cli",
    version,
    about = "Thin CLI adapter for text-analysis"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    Info {
        #[arg(long)]
        json: bool,
    },
    Schema {
        #[arg(long)]
        json: bool,
    },
    Operations {
        #[arg(long)]
        json: bool,
    },
    Run {
        #[arg(long, default_value = "analysis.describe")]
        operation: String,
        #[arg(long)]
        json: Option<String>,
        #[arg(long)]
        file: Option<String>,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Info { json: false }) {
        Command::Info { json } => print_payload(
            json,
            "text-analysis",
            &text_analysis_cli::package_metadata_json(),
        ),
        Command::Schema { json } => print_payload(
            json,
            "text-analysis command schema",
            &text_analysis_cli::command_schema_json(),
        ),
        Command::Operations { json } => {
            let payload = serde_json::to_string(&text_analysis_cli::package_surface().operations)?;
            print_payload(json, "text-analysis operations", &payload);
        }
        Command::Run {
            operation,
            json,
            file,
        } => {
            let input = read_input(json, file)?;
            let response = text_analysis_cli::run_operation(&operation, input)
                .map_err(std::io::Error::other)?;
            println!("{}", serde_json::to_string(&response)?);
        }
    }
    Ok(())
}

fn read_input(
    json: Option<String>,
    file: Option<String>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let input = if let Some(json) = json {
        json
    } else if let Some(file) = file {
        fs::read_to_string(file)?
    } else {
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer)?;
        if buffer.trim().is_empty() {
            "{}".to_string()
        } else {
            buffer
        }
    };
    Ok(serde_json::from_str(&input)?)
}

fn print_payload(json: bool, title: &str, payload: &str) {
    if json {
        println!("{payload}");
    } else {
        println!("{title}");
        println!("{payload}");
    }
}
