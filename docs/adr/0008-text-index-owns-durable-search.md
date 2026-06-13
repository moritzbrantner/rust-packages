# ADR 0008: Text Index Owns Durable Search

## Status

Accepted.

## Context

`text-retrieval` historically owned chunking, lexical search, vector search,
hybrid ranking, and JSON/JSONL persistence helpers. That worked for transient
retrieval workflows, but large local corpora need a clearer boundary for durable
indexes, SQLite schema ownership, searchable semantic facets, and package
surfaces that can describe persistence side effects.

The text package family now distinguishes a raw Text Corpus from a searchable
Text Index. `text-analysis::TextWorkspace` remains the primary package-consumer
workflow, while lower-level crates keep focused responsibilities.

## Decision

Introduce `moritzbrantner-text-index` as the long-term owner of durable text
indexing and search. It owns deterministic chunking, in-memory indexing,
SQLite-backed persistence behind the `sqlite` feature, lexical/semantic/hybrid
query contracts, metadata/source/time/provenance filters, semantic facets,
analysis attachments, inspection reports, and snapshot planning.

`text-retrieval` transitions to contract ingestion plus compatibility wrappers
for existing retrieval APIs. Existing `SearchDocument`, `DocumentChunk`,
`RetrievalIndex`, `SearchQuery`, `SearchResult`, and `PersistedSearchIndex`
surfaces stay available where practical. New durable indexing/search
development belongs in `text-index`.

SQLite is the first durable backend. Default builds remain deterministic and
no-network; the default semantic backend is deterministic hashed embeddings.

## Consequences

Compatibility wrappers stay so existing package consumers can migrate gradually.
Docs and package matrices must describe the transitional split: `text-index`
owns durable indexes; `text-retrieval` owns contract ingestion, legacy retrieval
compatibility, and reranking APIs.

Package examples remain transient and dry-run by default. Durable SQLite writes
require an explicit path plus `commit: true`; browser/WASM surfaces report
SQLite as unsupported rather than pretending to persist.
