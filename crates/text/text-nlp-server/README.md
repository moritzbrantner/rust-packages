# text-nlp-server

HTTP API adapter for `text-nlp-tasks`.

Run:

```bash
cargo run -p text-nlp-server -- --addr 127.0.0.1:3000
```

Endpoints:

- `GET /health`
- `GET /api/package`
- `GET /api/schema`
- `GET /api/models`
- `GET /api/models/:task`
- `POST /api/classify`
- `POST /api/sentiment`
- `POST /api/embed`
- `POST /api/zero-shot`
- `POST /api/summarize`
- `POST /api/rerank`
- `POST /api/question-answer`
- `POST /api/run`
