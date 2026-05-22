use clap::{Parser, Subcommand, ValueEnum};
use std::fs;
use std::path::PathBuf;
use text_nlp_models::{
    analyze_sentiment, answer_question, classify_text, embed_texts, model_catalog, rerank,
    summarize, zero_shot_classify, EmbeddingRequest, FallbackPolicy, ImportedPrediction,
    ModelSelection, QuestionAnsweringRequest, RerankRequest, SentimentRequest, SummaryRequest,
    SummaryStrategy, TextClassificationRequest, ZeroShotClassificationRequest,
};

#[derive(Debug, Parser)]
#[command(name = "text-nlp", version, about = "Shared NLP task CLI")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// List known NLP model presets.
    Models {
        /// Emit only models for a task path segment, for example sentiment or embed.
        #[arg(long)]
        task: Option<String>,
    },
    /// Classify text.
    Classify {
        /// Text to classify.
        #[arg(long)]
        text: String,
        /// Comma-separated labels for fallback classification.
        #[arg(long)]
        labels: Option<String>,
        /// Model id or preset id.
        #[arg(long)]
        model: Option<String>,
        /// Fallback policy.
        #[arg(long, value_enum, default_value_t = FallbackArg::Lexical)]
        fallback: FallbackArg,
    },
    /// Analyze sentiment.
    Sentiment {
        /// Text to analyze.
        #[arg(long)]
        text: String,
        /// Model id or preset id.
        #[arg(long)]
        model: Option<String>,
        /// Fallback policy.
        #[arg(long, value_enum, default_value_t = FallbackArg::Lexical)]
        fallback: FallbackArg,
    },
    /// Embed one or more texts.
    Embed {
        /// Text to embed. Repeat for multiple inputs.
        #[arg(long)]
        text: Vec<String>,
        /// UTF-8 file to embed as a single input.
        #[arg(long)]
        file: Option<PathBuf>,
        /// Fallback vector dimensions.
        #[arg(long, default_value_t = 384)]
        dimensions: usize,
        /// Model id or preset id.
        #[arg(long)]
        model: Option<String>,
        /// Fallback policy.
        #[arg(long, value_enum, default_value_t = FallbackArg::Fast)]
        fallback: FallbackArg,
    },
    /// Classify text against caller-supplied labels.
    ZeroShot {
        /// Text to classify.
        #[arg(long)]
        text: String,
        /// Comma-separated candidate labels.
        #[arg(long)]
        labels: String,
        /// Model id or preset id.
        #[arg(long)]
        model: Option<String>,
        /// Fallback policy.
        #[arg(long, value_enum, default_value_t = FallbackArg::Lexical)]
        fallback: FallbackArg,
    },
    /// Summarize text.
    Summarize {
        /// Text to summarize.
        #[arg(long)]
        text: Option<String>,
        /// UTF-8 file to summarize.
        #[arg(long)]
        file: Option<PathBuf>,
        /// Maximum sentence count.
        #[arg(long, default_value_t = 3)]
        max_sentences: usize,
        /// Use lexical extraction instead of embedding-extractive mode.
        #[arg(long)]
        lexical: bool,
    },
    /// Rerank documents against a query.
    Rerank {
        /// Query text.
        #[arg(long)]
        query: String,
        /// Document text. Repeat for multiple documents.
        #[arg(long)]
        document: Vec<String>,
        /// JSON file containing an array of document strings.
        #[arg(long)]
        documents: Option<PathBuf>,
        /// Maximum result count.
        #[arg(long, default_value_t = 10)]
        top_k: usize,
        /// Fallback policy.
        #[arg(long, value_enum, default_value_t = FallbackArg::Lexical)]
        fallback: FallbackArg,
    },
    /// Answer a question from a context using imported span predictions.
    QuestionAnswer {
        /// Question text.
        #[arg(long)]
        question: String,
        /// Context text.
        #[arg(long)]
        context: Option<String>,
        /// UTF-8 context file.
        #[arg(long)]
        context_file: Option<PathBuf>,
        /// JSON file with imported span predictions.
        #[arg(long)]
        imported_predictions: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum FallbackArg {
    Error,
    Fast,
    Lexical,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let payload = match cli.command.unwrap_or(Command::Models { task: None }) {
        Command::Models { task } => {
            let task = task.as_deref().and_then(text_nlp_models::parse_task);
            serde_json::to_value(model_catalog(task))?
        }
        Command::Classify {
            text,
            labels,
            model,
            fallback,
        } => serde_json::to_value(classify_text(TextClassificationRequest {
            text,
            labels: split_csv(labels.as_deref()),
            top_k: 3,
            multi_label: false,
            model: selection(model, fallback),
            imported_predictions: Vec::new(),
        })?)?,
        Command::Sentiment {
            text,
            model,
            fallback,
        } => serde_json::to_value(analyze_sentiment(SentimentRequest {
            text,
            model: selection(model, fallback),
            imported_predictions: Vec::new(),
        })?)?,
        Command::Embed {
            text,
            file,
            dimensions,
            model,
            fallback,
        } => {
            let mut texts = text;
            if let Some(path) = file {
                texts.push(fs::read_to_string(path)?);
            }
            serde_json::to_value(embed_texts(EmbeddingRequest {
                texts,
                model: selection(model, fallback),
                dimensions,
                normalize: true,
                imported_embeddings: Vec::new(),
            })?)?
        }
        Command::ZeroShot {
            text,
            labels,
            model,
            fallback,
        } => serde_json::to_value(zero_shot_classify(ZeroShotClassificationRequest {
            text,
            labels: split_csv(Some(&labels)),
            hypothesis_template: "This example is about {}.".to_string(),
            model: selection(model, fallback),
            imported_predictions: Vec::new(),
        })?)?,
        Command::Summarize {
            text,
            file,
            max_sentences,
            lexical,
        } => {
            let text = match (text, file) {
                (Some(text), _) => text,
                (None, Some(path)) => fs::read_to_string(path)?,
                (None, None) => return Err("summarize requires --text or --file".into()),
            };
            serde_json::to_value(summarize(SummaryRequest {
                text,
                max_sentences,
                strategy: if lexical {
                    SummaryStrategy::LexicalExtractive
                } else {
                    SummaryStrategy::EmbeddingExtractive
                },
                model: ModelSelection::default(),
                imported_sentence_embeddings: Vec::new(),
            })?)?
        }
        Command::Rerank {
            query,
            document,
            documents,
            top_k,
            fallback,
        } => {
            let mut docs = document;
            if let Some(path) = documents {
                docs.extend(serde_json::from_str::<Vec<String>>(&fs::read_to_string(
                    path,
                )?)?);
            }
            serde_json::to_value(rerank(RerankRequest {
                query,
                documents: docs,
                top_k,
                model: selection(None, fallback),
                imported_scores: Vec::new(),
            })?)?
        }
        Command::QuestionAnswer {
            question,
            context,
            context_file,
            imported_predictions,
        } => {
            let context = match (context, context_file) {
                (Some(context), _) => context,
                (None, Some(path)) => fs::read_to_string(path)?,
                (None, None) => {
                    return Err("question-answer requires --context or --context-file".into())
                }
            };
            let imported_predictions = match imported_predictions {
                Some(path) => {
                    serde_json::from_str::<Vec<ImportedPrediction>>(&fs::read_to_string(path)?)?
                }
                None => Vec::new(),
            };
            serde_json::to_value(answer_question(QuestionAnsweringRequest {
                question,
                context,
                top_k: 3,
                model: ModelSelection::default(),
                imported_predictions,
            })?)?
        }
    };

    println!("{}", serde_json::to_string(&payload)?);
    Ok(())
}

fn selection(model_id: Option<String>, fallback: FallbackArg) -> ModelSelection {
    ModelSelection {
        model_id,
        runtime: None,
        fallback_policy: match fallback {
            FallbackArg::Error => FallbackPolicy::Error,
            FallbackArg::Fast => FallbackPolicy::FastFallback,
            FallbackArg::Lexical => FallbackPolicy::LexicalFallback,
        },
    }
}

fn split_csv(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}
