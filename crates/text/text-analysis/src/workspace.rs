use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use text_core::{TextDocumentContract, TextSegmentContract};
use text_embeddings::{HashedTextEmbedder, TextEmbeddingConfig};
use text_lexical::{CorpusOptions, TextCorpus, TextCorpusDocument};
use text_retrieval::{
    IngestReport, IngestionOptions, RetrievalIndex, SearchDocument, SearchQuery, SearchResult,
};
use video_analysis_core::{DetectError, Result};

use crate::{
    analyze_corpus, analyze_document, CorpusAnalysisOptions, CorpusAnalysisReport,
    DocumentAnalysisOptions, DocumentAnalysisReport,
};

#[derive(Debug, Clone, PartialEq)]
pub struct TextWorkspaceOptions {
    pub corpus: CorpusOptions,
    pub ingestion: IngestionOptions,
    pub document_analysis: DocumentAnalysisOptions,
    pub corpus_analysis: CorpusAnalysisOptions,
    pub embedding_dimensions: usize,
}

impl Default for TextWorkspaceOptions {
    fn default() -> Self {
        Self {
            corpus: CorpusOptions::default(),
            ingestion: IngestionOptions::default(),
            document_analysis: DocumentAnalysisOptions::default(),
            corpus_analysis: CorpusAnalysisOptions::default(),
            embedding_dimensions: 128,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "value")]
pub enum WorkspaceDocument {
    DocumentContract(TextDocumentContract),
    SegmentContract(TextSegmentContract),
    SearchDocument(SearchDocument),
    #[cfg(feature = "transcripts")]
    Transcription(text_transcripts::TranscriptionContract),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceIngestReport {
    pub documents_received: usize,
    pub documents_added: usize,
    pub documents_replaced: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSearchReport {
    pub query: String,
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSnapshot {
    pub document_count: usize,
    pub has_retrieval_index: bool,
    pub cached_analysis_reports: usize,
    pub documents: Vec<TextDocumentContract>,
}

#[derive(Debug, Clone)]
pub struct TextWorkspace {
    options: TextWorkspaceOptions,
    corpus: TextCorpus,
    retrieval: Option<RetrievalIndex<HashedTextEmbedder>>,
    analysis_reports: BTreeMap<String, DocumentAnalysisReport>,
}

impl Default for TextWorkspace {
    fn default() -> Self {
        Self::new(TextWorkspaceOptions::default())
    }
}

impl TextWorkspace {
    pub fn new(options: TextWorkspaceOptions) -> Self {
        Self {
            corpus: TextCorpus::new(options.corpus.clone()),
            options,
            retrieval: None,
            analysis_reports: BTreeMap::new(),
        }
    }

    pub fn corpus(&self) -> &TextCorpus {
        &self.corpus
    }

    pub fn retrieval_index(&self) -> Option<&RetrievalIndex<HashedTextEmbedder>> {
        self.retrieval.as_ref()
    }

    pub fn ingest_documents<I>(&mut self, documents: I) -> Result<WorkspaceIngestReport>
    where
        I: IntoIterator<Item = WorkspaceDocument>,
    {
        let mut received = 0;
        let mut added = 0;
        let mut replaced = 0;
        for document in documents {
            received += 1;
            for corpus_document in workspace_document_to_corpus_documents(document) {
                if self.corpus.document(&corpus_document.id).is_some() {
                    self.corpus.replace_document(corpus_document)?;
                    replaced += 1;
                } else {
                    self.corpus.add_document(corpus_document)?;
                    added += 1;
                }
            }
        }
        self.retrieval = None;
        Ok(WorkspaceIngestReport {
            documents_received: received,
            documents_added: added,
            documents_replaced: replaced,
        })
    }

    pub fn analyze_document(&mut self, id: &str) -> Result<DocumentAnalysisReport> {
        let document = self
            .corpus
            .document(id)
            .ok_or_else(|| invalid_argument(format!("document `{id}` was not found")))?;
        let contract = TextDocumentContract {
            id: document.id.clone(),
            text: document.text.clone(),
            language: document.language.clone(),
            timestamp: document.timestamp,
            attributes: document.metadata.clone(),
            source: document.source.clone(),
            provenance: document.provenance.clone(),
            annotations: document.annotations.clone(),
        };
        let text_document = text_core::TextDocument {
            id: &contract.id,
            text: &contract.text,
            language: contract.language.as_deref(),
            timestamp: contract.timestamp.map(Into::into),
        };
        let report = analyze_document(&text_document, &self.options.document_analysis)?;
        self.analysis_reports
            .insert(report.id.clone(), report.clone());
        Ok(report)
    }

    pub fn analyze_corpus(&self) -> Result<CorpusAnalysisReport> {
        analyze_corpus(
            self.corpus.as_text_documents(),
            &self.options.corpus_analysis,
        )
    }

    pub fn build_retrieval_index(&mut self) -> Result<IngestReport> {
        let embedder = HashedTextEmbedder::new(
            TextEmbeddingConfig {
                dimensions: self.options.embedding_dimensions.max(1),
                use_idf: false,
            },
            self.options.corpus.clone(),
        )?;
        let mut index = RetrievalIndex::new(embedder);
        let documents = SearchDocument::from_text_corpus(&self.corpus);
        let report = index.ingest_documents(&documents, &self.options.ingestion)?;
        self.retrieval = Some(index);
        Ok(report)
    }

    pub fn search(&self, query: SearchQuery) -> Result<WorkspaceSearchReport> {
        let index = self
            .retrieval
            .as_ref()
            .ok_or_else(|| invalid_argument("retrieval index has not been built"))?;
        let results = index.search(&query)?;
        Ok(WorkspaceSearchReport {
            query: query.text,
            results,
        })
    }

    pub fn snapshot(&self) -> WorkspaceSnapshot {
        WorkspaceSnapshot {
            document_count: self.corpus.len(),
            has_retrieval_index: self.retrieval.is_some(),
            cached_analysis_reports: self.analysis_reports.len(),
            documents: self.corpus.text_document_contracts(),
        }
    }
}

fn workspace_document_to_corpus_documents(document: WorkspaceDocument) -> Vec<TextCorpusDocument> {
    match document {
        WorkspaceDocument::DocumentContract(document) => {
            vec![text_corpus_document_from_contract(&document)]
        }
        WorkspaceDocument::SegmentContract(segment) => {
            vec![text_corpus_document_from_contract(
                &segment.to_text_document_contract(),
            )]
        }
        WorkspaceDocument::SearchDocument(document) => {
            vec![text_corpus_document_from_search_document(document)]
        }
        #[cfg(feature = "transcripts")]
        WorkspaceDocument::Transcription(transcription) => transcription
            .segments
            .iter()
            .map(TextSegmentContract::from)
            .map(|segment| text_corpus_document_from_contract(&segment.to_text_document_contract()))
            .collect(),
    }
}

fn text_corpus_document_from_contract(document: &TextDocumentContract) -> TextCorpusDocument {
    let mut corpus_document = TextCorpusDocument::new(&document.id, &document.text);
    corpus_document.language = document.language.clone();
    corpus_document.timestamp = document.timestamp;
    corpus_document.source = document.source.clone();
    corpus_document.provenance = document.provenance.clone();
    corpus_document.annotations = document.annotations.clone();
    corpus_document.metadata = document.attributes.clone();
    corpus_document
}

fn text_corpus_document_from_search_document(document: SearchDocument) -> TextCorpusDocument {
    let mut corpus_document = TextCorpusDocument::new(document.id, document.body);
    corpus_document.language = document.metadata.get("language").cloned();
    corpus_document.source = document.source;
    corpus_document.provenance = document.provenance;
    corpus_document.annotations = document.annotations;
    corpus_document.metadata = document.metadata;
    corpus_document
}

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}
