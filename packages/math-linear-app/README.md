# math-linear-app

React, TypeScript, TailwindCSS, Bun, and oxfmt frontend for `math-linear`.
The workbench exposes the Analytical Math Crates matrix surface, including SVD,
pseudoinverse, rank, precision, tolerance, and thin-factor controls from
operation metadata.

Run the server:

```bash
cargo run -p math-linear-server -- --addr 127.0.0.1:3000
```

Run the app:

```bash
bun run --cwd packages/math-linear-app dev
```
