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

## Benchmarks

### Python parse (`rjson` vs others)

Full parse into a Python object tree (`dict` / `list` / …).

| Parser | median | MB/s | vs `rjson` (wall) |
|---|---:|---:|---:|
| `rjson.rjson_loads` | 136 ms | 37 | 1.00× (baseline) |
| `rjson.rjson_load` | 135 ms | 37 | 1.00× |
| `json.loads` | 37 ms | 136 | **0.27×** (faster than rjson) |
| `json.loads(bytes)` | 35 ms | 143 | **0.26×** |
| `ujson.loads` | 35 ms | 142 | **0.26×** |
| `ujson.loads(bytes)` | 33 ms | 150 | **0.25×** |
| `orjson.loads` | 24 ms | 213 | **0.17×** |
| `orjson.loads(bytes)` | 22 ms | 228 | **0.16×** |

### CLI parse (`rs-jq` vs stock `jq`)

| Tool | median | MB/s | tokens/s | peak RSS | vs `rs-jq` (wall) |
|---|---:|---:|---:|---:|---:|
| `rs-jq` (`target/release/jq`) | 190 ms | 26 | 8.1M | ~107 MiB | 1.00× (baseline) |
| stock `jq` | 300 ms | 17 | 5.1M | ~67 MiB | **1.58×** (slower than rs-jq) |

## Notable JQL queries

Queries below run against the fixture `src/jql/sample.json`.

### Access

```text
.                              # whole document
.count                         # 42
.jobs[1]                       # second job
.jobs[0,2]                     # pick indices
.jobs[1:3]                     # slice [start:end)
.jobs[]                        # all items (identity on array)
.person{name,age}              # project object keys
.jobs[]{title,description}     # project keys on each item
.company.teams[0].name         # "platform"
.matrix[0][1]                  # nested index → 2
.nested[1][1][0]               # deep nest → 3
```

### Pipe

```text
.matrix[0] | .[1]                              # 2
.company | .teams[1] | .name                   # "data"
.jobs | .[0].title                             # "Dev"
.matrix[1] | .[0,2]                            # [4, 6]
.count | . > 40                                # true
.jobs[0].title | . == "Dev"                    # true
```

### Filter

```text
.jobs | filter(.active == true)
.jobs | filter(.id > 2)
.people | filter(.vip == true && .score >= 70)
.candidates | filter(.id > 10 && .age < 20 && (.money > 10 || .gold >= 1))
.jobs | filter((.level == "senior" || .level == "mid") && .money >= 50)
```

### Compose

```text
.jobs[]{title,id} | filter(.id > 2) | .[0].title
.company.teams | filter(.size > 5) | .[0].name
.jobs[0,3] | filter(.active == true) | .[1].title
.people | filter(.vip == true) | .[0].name
.matrix | filter(.[0] > 3) | .[0].[1]
```
