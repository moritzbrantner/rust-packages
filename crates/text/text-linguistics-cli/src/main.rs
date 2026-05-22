use clap::{Parser, Subcommand, ValueEnum};
use std::fs;
use std::path::PathBuf;
use text_linguistics as _;
use text_linguistics::{
    analyze_text, AnalysisProfile, EntityRecognitionMode, LinguisticAnalysis,
    LinguisticAnalysisOptions, TextNlpConfig, TextNlpPipeline,
};

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
        #[command(flatten)]
        analysis: AnalysisArgs,
        /// Emit machine-readable JSON.
        #[arg(long, default_value_t = true)]
        json: bool,
    },
    /// Analyze a UTF-8 text file and emit JSON.
    AnalyzeFile {
        /// File to analyze.
        path: PathBuf,
        #[command(flatten)]
        analysis: AnalysisArgs,
        /// Emit machine-readable JSON.
        #[arg(long, default_value_t = true)]
        json: bool,
    },
}

#[derive(Debug, Clone, Parser)]
struct AnalysisArgs {
    /// Analysis profile to run.
    #[arg(long, value_enum, default_value_t = ProfileArg::Rich)]
    profile: ProfileArg,
    /// Named entity backend.
    #[arg(long, value_enum, default_value_t = EntityRecognitionArg::LocalModel)]
    entity_recognition: EntityRecognitionArg,
    /// Directory containing downloaded model bundles.
    #[arg(long, default_value = ".video-analysis-models")]
    model_dir: PathBuf,
    /// Do not download the BERT-NER bundle if it is missing.
    #[arg(long)]
    no_auto_download: bool,
    /// Show model download progress when auto-download is enabled.
    #[arg(long, default_value_t = true)]
    download_progress: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum ProfileArg {
    Fast,
    Balanced,
    Rich,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum EntityRecognitionArg {
    LocalModel,
    Heuristic,
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
        Command::Analyze {
            text,
            analysis,
            json,
        } => {
            let payload = analysis_json(&text, &analysis)?;
            print_payload(json, "text-linguistics analysis", &payload);
        }
        Command::AnalyzeFile {
            path,
            analysis,
            json,
        } => {
            let text = fs::read_to_string(path)?;
            let payload = analysis_json(&text, &analysis)?;
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

fn analysis_json(text: &str, args: &AnalysisArgs) -> Result<String, Box<dyn std::error::Error>> {
    let (analysis, model_metadata) = run_analysis(text, args)?;
    Ok(analysis_payload(&analysis, model_metadata).to_string())
}

#[derive(Debug, Clone, Copy)]
struct ModelMetadata {
    entity_recognition: &'static str,
    entity_model: Option<&'static str>,
}

fn run_analysis(
    text: &str,
    args: &AnalysisArgs,
) -> Result<(LinguisticAnalysis, ModelMetadata), Box<dyn std::error::Error>> {
    if args.entity_recognition == EntityRecognitionArg::Heuristic {
        let analysis = analyze_text(text, &LinguisticAnalysisOptions::heuristic())?;
        return Ok((
            analysis,
            ModelMetadata {
                entity_recognition: "heuristic",
                entity_model: None,
            },
        ));
    }

    let mut config = config_for_profile(args.profile);
    config.options.entity_recognition.mode = EntityRecognitionMode::LocalModel;
    config.options.entity_recognition.bundle_dir = args.model_dir.clone();
    config.options.entity_recognition.auto_download = !args.no_auto_download;
    config.options.entity_recognition.download_progress = args.download_progress;

    let model_metadata = model_metadata_for_config(&config);
    let analysis = TextNlpPipeline::new(config).analyze_text(text)?;
    Ok((analysis, model_metadata))
}

fn config_for_profile(profile: ProfileArg) -> TextNlpConfig {
    match profile {
        ProfileArg::Fast => TextNlpConfig::fast(),
        ProfileArg::Balanced => TextNlpConfig::balanced(),
        ProfileArg::Rich => TextNlpConfig::rich(),
    }
}

fn model_metadata_for_config(config: &TextNlpConfig) -> ModelMetadata {
    if matches!(config.profile, AnalysisProfile::Fast) {
        ModelMetadata {
            entity_recognition: "heuristic",
            entity_model: None,
        }
    } else {
        ModelMetadata {
            entity_recognition: "local-model",
            entity_model: Some("bert-base-ner"),
        }
    }
}

fn analysis_payload(
    analysis: &LinguisticAnalysis,
    model_metadata: ModelMetadata,
) -> serde_json::Value {
    serde_json::json!({
        "package": "text-linguistics-cli",
        "library": "text-linguistics",
        "accepted": true,
        "operation": "analyze",
        "profile": format!("{:?}", analysis.profile),
        "provenance": format!("{:?}", analysis.provenance),
        "confidence": analysis.confidence.get(),
        "model": {
            "entityRecognition": model_metadata.entity_recognition,
            "entityModel": model_metadata.entity_model,
            "tokenizerMode": format!("{:?}", analysis.tokenizer.mode),
            "tokenizerSource": analysis.tokenizer.source.as_ref().map(|source| format!("{source:?}")),
            "alignmentCount": analysis.alignments.as_ref().map(|alignment| alignment.aligned_tokens.len()).unwrap_or(0)
        },
        "summary": {
            "language": analysis.language.primary.as_ref().map(|prediction| prediction.language.as_str()),
            "tokenCount": analysis.tokens.len(),
            "sentenceCount": analysis.sentences.len(),
            "lemmaCount": analysis.lemmas.len(),
            "entityCount": analysis.entities.len(),
            "eventCount": analysis.events.len(),
            "relationCount": analysis.relations.len(),
            "topicCount": analysis.topics.descriptors.len(),
            "chunkCount": analysis.chunks.len()
        },
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
                "sentenceIndex": entity.sentence_index,
                "tokenStart": entity.token_start,
                "tokenEnd": entity.token_end,
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
}
