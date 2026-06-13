# moritzbrantner-text-index

Durable local text indexing and hybrid search for the text package family.

This crate owns the long-term text index boundary: deterministic chunking,
in-memory indexing, SQLite-backed persistence behind the `sqlite` feature,
lexical/semantic/hybrid search, metadata filters, semantic facets, and
analysis attachment persistence. The default build is deterministic and
no-network; hashed embeddings are the default semantic backend.
