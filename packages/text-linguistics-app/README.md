# text-linguistics-app

React workbench for `text-linguistics` language detection, lemmas, POS tags,
entities, events, relations, topics, and style metadata.

In the overview app it uses the aggregate Rust server package route. Standalone,
run the package server first:

```bash
cargo run -p text-linguistics-server -- --addr 127.0.0.1:3000
bun run --cwd packages/text-linguistics-app dev
```
