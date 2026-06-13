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
indexing and search. It owns generic ingestion from text contracts,
`TextCorpus`, and caller-supplied index records, plus deterministic chunking,
in-memory indexing, SQLite-backed persistence behind the `sqlite` feature,
lexical/semantic/hybrid query contracts, metadata/source/time/provenance
filters, semantic facets, analysis attachments, inspection reports, and
snapshot planning.

`text-retrieval` transitions away from owning generic ingestion and toward
soft-legacy compatibility wrappers for existing retrieval APIs. Existing `SearchDocument`,
`DocumentChunk`,
`RetrievalIndex`, `SearchQuery`, `SearchResult`, and `PersistedSearchIndex`
surfaces stay available where practical as soft-legacy compatibility. New
durable indexing/search development belongs in `text-index`; retrieval keeps
older adapters, legacy snapshot import paths, and reranking.

SQLite is the first durable backend. Default builds remain deterministic and
no-network; the default semantic backend is deterministic hashed embeddings.

## Consequences

Compatibility wrappers stay so existing package consumers can migrate gradually.
Docs and package matrices must describe the transitional split: `text-index`
owns durable indexes and generic ingestion; `text-retrieval` keeps older
compatibility adapters, soft-legacy retrieval compatibility, snapshot import,
and reranking APIs.

Package examples remain transient and dry-run by default. Durable SQLite writes
require an explicit path plus `commit: true`; browser/WASM surfaces report
SQLite as unsupported rather than pretending to persist. CLI/server/WASM/app
package surfaces are request-scoped; server-side index sessions and open index
handles are out of scope.
