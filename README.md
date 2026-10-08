# rs-jq

Learning project: JSON parser + jq-like query language, written in Rust.

Still early / WIP. APIs and behavior will change.

## What's here

- `src/json` — JSON lexer & parser
- `src/jql` — query language (lexer, parser, engine)
- `src/bin/jq` — CLI binary (optional, feature `cli`)

## Build

```bash
cargo build
```

CLI:

```bash
cargo build --features cli
```

## Test

```bash
cargo test
```

## Status

Rough notes for now:

- JSON parsing works for the basics
- JQL supports field/index access, pipelines, and some `filter(...)` conditions
- Not trying to be fully jq-compatible yet

See `src/json/readme.md` and `src/jql/readme.md` for grammar notes.
