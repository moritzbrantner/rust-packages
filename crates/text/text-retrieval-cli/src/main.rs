use clap::{Parser, Subcommand};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use text_embeddings::{HashedTextEmbedder, TextEmbeddingConfig};
use text_lexical::CorpusOptions;
use text_retrieval as _;
use text_retrieval::{
    IngestionOptions, PersistedSearchIndex, RetrievalIndex, RetrievalMode, SearchDocument,
    SearchQuery,
};

#[derive(Debug, Parser)]
#[command(
    name = "text-retrieval-cli",
    version,
    about = "Thin CLI adapter for text-retrieval"
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
    /// Ingest JSONL search documents and persist an index directory.
    Ingest {
        /// Input JSONL file containing SearchDocument records.
        #[arg(long)]
        input: PathBuf,
        /// Output directory for the persisted retrieval index.
        #[arg(long)]
        output: PathBuf,
    },
    /// Search a persisted retrieval index.
    Search {
        /// Persisted retrieval index directory.
        #[arg(long)]
        index: PathBuf,
        /// Query text.
        #[arg(long)]
        query: String,
        /// Maximum result count.
        #[arg(long, default_value_t = 10)]
        top_k: usize,
        /// Search mode: full-text, semantic, or hybrid.
        #[arg(long, default_value = "hybrid")]
        mode: String,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Info { json: false }) {
        Command::Info { json } => print_payload(
            json,
            "text-retrieval",
            &text_retrieval_cli::package_metadata_json(),
        ),
        Command::Schema { json } => print_payload(
            json,
            "text-retrieval command schema",
            &text_retrieval_cli::command_schema_json(),
        ),
        Command::Ingest { input, output } => {
            let documents = read_documents(input)?;
            let embedder = default_embedder()?;
            let mut index = RetrievalIndex::new(embedder);
            let report = index.ingest_documents(&documents, &IngestionOptions::default())?;
            PersistedSearchIndex::from_index(&index).save_to_path(&output)?;
            println!("{}", serde_json::to_string(&report)?);
        }
        Command::Search {
            index,
            query,
            top_k,
            mode,
        } => {
            let embedder = default_embedder()?;
            let index = PersistedSearchIndex::load_with_embedder(&index, embedder)?;
            let query = SearchQuery::new(query, top_k).mode(parse_mode(&mode)?);
            let results = index.search(&query)?;
            println!("{}", serde_json::to_string(&results)?);
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

fn read_documents(path: PathBuf) -> Result<Vec<SearchDocument>, Box<dyn std::error::Error>> {
    let reader = BufReader::new(File::open(path)?);
    let mut documents = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        documents.push(serde_json::from_str(&line)?);
    }
    Ok(documents)
}

fn default_embedder() -> Result<HashedTextEmbedder, Box<dyn std::error::Error>> {
    Ok(HashedTextEmbedder::new(
        TextEmbeddingConfig {
            dimensions: 128,
            use_idf: true,
        },
        CorpusOptions::default(),
    )?)
}

fn parse_mode(value: &str) -> Result<RetrievalMode, Box<dyn std::error::Error>> {
    match value {
        "full-text" | "full_text" => Ok(RetrievalMode::FullText),
        "semantic" => Ok(RetrievalMode::Semantic),
        "hybrid" => Ok(RetrievalMode::Hybrid),
        other => Err(format!("unsupported retrieval mode `{other}`").into()),
    }
}
