use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use text_linguistics as _;
use text_linguistics::{analyze_text, LinguisticAnalysisOptions};

#[derive(Debug, Parser)]
#[command(
    name = "text-linguistics-cli",
    version,
    about = "Thin CLI adapter for text-linguistics"
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
    /// Analyze supplied text and emit JSON.
    Analyze {
        /// Text to analyze.
        #[arg(long)]
        text: String,
        /// Emit machine-readable JSON.
        #[arg(long, default_value_t = true)]
        json: bool,
    },
    /// Analyze a UTF-8 text file and emit JSON.
    AnalyzeFile {
        /// File to analyze.
        path: PathBuf,
        /// Emit machine-readable JSON.
        #[arg(long, default_value_t = true)]
        json: bool,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Info { json: false }) {
        Command::Info { json } => print_payload(
            json,
            "text-linguistics",
            &text_linguistics_cli::package_metadata_json(),
        ),
        Command::Schema { json } => print_payload(
            json,
            "text-linguistics command schema",
            &text_linguistics_cli::command_schema_json(),
        ),
        Command::Analyze { text, json } => {
            let payload = analysis_json(&text)?;
            print_payload(json, "text-linguistics analysis", &payload);
        }
        Command::AnalyzeFile { path, json } => {
            let text = fs::read_to_string(path)?;
            let payload = analysis_json(&text)?;
            print_payload(json, "text-linguistics analysis", &payload);
        }
    }
    Ok(())
}

fn print_payload(json: bool, title: &str, payload: &str) {
    if json {
        println!("{payload}");
    } else {
        println!("{title}");
        println!("{payload}");
    }
}

fn analysis_json(text: &str) -> Result<String, Box<dyn std::error::Error>> {
    let analysis = analyze_text(text, &LinguisticAnalysisOptions::default())?;
    Ok(serde_json::json!({
        "language": analysis.language.primary.as_ref().map(|prediction| prediction.language.as_str()),
        "tokenCount": analysis.tokens.len(),
        "sentenceCount": analysis.sentences.len(),
        "lemmaCount": analysis.lemmas.len(),
        "entityCount": analysis.entities.len(),
        "eventCount": analysis.events.len(),
        "entities": analysis.entities.iter().map(|entity| {
            serde_json::json!({
                "id": entity.id,
                "text": entity.mention.text,
                "normalized": entity.normalized,
                "kind": format!("{:?}", entity.entity_type),
                "confidence": entity.confidence,
            })
        }).collect::<Vec<_>>(),
        "events": analysis.events.iter().map(|event| {
            serde_json::json!({
                "sentenceIndex": event.sentence_index,
                "predicate": event.predicate,
                "lemma": event.lemma,
                "confidence": event.confidence,
            })
        }).collect::<Vec<_>>(),
    })
    .to_string())
}
