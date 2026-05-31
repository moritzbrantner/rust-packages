use std::fs;
use std::io::Read;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "geo-io-osm-cli",
    version,
    about = "Thin CLI adapter for geo-io-osm"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print package and adapter metadata.
    Metadata {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Print package and adapter metadata.
    Info {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Print the command schema.
    Schema {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Print library operations.
    Operations {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Run one library-owned operation.
    Run {
        /// Operation id.
        #[arg(long, default_value = "describe")]
        operation: String,
        /// JSON request payload.
        #[arg(long)]
        json: Option<String>,
        /// Read JSON request payload from a file.
        #[arg(long)]
        file: Option<String>,
    },
    /// Filter an OSM PBF file and write GeoJSON.
    Filter {
        /// Input .osm.pbf file.
        #[arg(long)]
        input: PathBuf,
        /// JSON filter spec file.
        #[arg(long)]
        spec: PathBuf,
        /// Output .geojson file.
        #[arg(long)]
        output: PathBuf,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Metadata { json: false }) {
        Command::Metadata { json } | Command::Info { json } => {
            print_payload(json, "geo-io-osm", &geo_io_osm_cli::package_metadata_json())
        }
        Command::Schema { json } => print_payload(
            json,
            "geo-io-osm command schema",
            &geo_io_osm_cli::command_schema_json(),
        ),
        Command::Operations { json } => {
            let payload = serde_json::to_string(&geo_io_osm_cli::package_surface().operations)?;
            print_payload(json, "geo-io-osm operations", &payload);
        }
        Command::Run {
            operation,
            json,
            file,
        } => {
            let input = read_input(json, file)?;
            let response =
                geo_io_osm_cli::run_operation(&operation, input).map_err(std::io::Error::other)?;
            println!("{}", serde_json::to_string(&response)?);
        }
        Command::Filter {
            input,
            spec,
            output,
        } => {
            let feature_count = geo_io_osm_cli::filter_to_geojson(&input, &spec, &output)
                .map_err(std::io::Error::other)?;
            println!(
                "{}",
                serde_json::json!({
                    "output": output,
                    "featureCount": feature_count
                })
            );
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
