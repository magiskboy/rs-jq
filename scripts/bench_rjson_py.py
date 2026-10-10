#!/usr/bin/env python3
"""Benchmark rs-jq Python bindings (rjson) vs common Python JSON parsers.

Compares in-process parse throughput of:
  - json.loads / json.loads(bytes)     — CPython stdlib
  - orjson.loads                       — orjson (Rust, via PyO3)
  - ujson.loads                        — ultrajson
  - rjson.rjson_loads / rjson.rjson_load — this project's PyO3 binding

Build the extension first:
  pip install maturin
  maturin develop --release

Optional deps for the extra parsers:
  pip install orjson ujson

Examples:
  python3 scripts/bench_rjson_py.py
  python3 scripts/bench_rjson_py.py --size-mb 10 --runs 5
  python3 scripts/bench_rjson_py.py --input path/to/data.json --mode loads
  python3 scripts/bench_rjson_py.py --only orjson,ujson
"""

from __future__ import annotations

import argparse
import json
import platform
import statistics
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Sequence


ROOT = Path(__file__).resolve().parents[1]

PARSER_CHOICES = ("json", "orjson", "ujson", "rjson")


# ---------------------------------------------------------------------------
# Fixture generation
# ---------------------------------------------------------------------------

def generate_json(target_bytes: int, seed: int = 1) -> str:
    """Build a compact JSON array of objects until size >= target_bytes."""
    items: list[dict[str, Any]] = []
    i = seed
    while True:
        batch = []
        for _ in range(200):
            batch.append(
                {
                    "id": i,
                    "name": f"user-{i}",
                    "active": i % 2 == 0,
                    "score": (i * 17) % 10_000 / 100.0,
                    "tags": [f"t{j}" for j in range(i % 5)],
                    "meta": {
                        "city": f"city-{i % 50}",
                        "level": i % 10,
                        "note": "x" * (8 + (i % 24)),
                    },
                }
            )
            i += 1
        items.extend(batch)
        payload = json.dumps(items, separators=(",", ":"))
        if len(payload.encode("utf-8")) >= target_bytes:
            return payload


def ensure_fixture(path: Path, size_mb: float, force: bool) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    target = int(size_mb * 1024 * 1024)
    if path.exists() and not force and path.stat().st_size >= target:
        return path
    print(f"generating fixture ~{size_mb:g} MiB → {path}", file=sys.stderr)
    path.write_text(generate_json(target), encoding="utf-8")
    return path


# ---------------------------------------------------------------------------
# Timing
# ---------------------------------------------------------------------------

@dataclass
class BenchStats:
    name: str
    walls: list[float]
    input_bytes: int
    errors: int = 0

    def median(self) -> float:
        return statistics.median(self.walls) if self.walls else float("nan")

    def mean(self) -> float:
        return statistics.mean(self.walls) if self.walls else float("nan")

    def stdev(self) -> float:
        return statistics.stdev(self.walls) if len(self.walls) >= 2 else 0.0

    def mb_per_s(self) -> float:
        t = self.median()
        return (self.input_bytes / (1024 * 1024)) / t if t > 0 else float("nan")


def bench(
    name: str,
    fn: Callable[[], Any],
    *,
    input_bytes: int,
    warmup: int,
    runs: int,
) -> BenchStats:
    print(f"\n==> {name}", file=sys.stderr)
    for i in range(warmup):
        t0 = time.perf_counter()
        try:
            fn()
        except Exception as exc:
            print(f"    warmup {i + 1} FAILED: {exc}", file=sys.stderr)
            return BenchStats(name=name, walls=[], input_bytes=input_bytes, errors=1)
        dt = time.perf_counter() - t0
        print(f"    warmup {i + 1}/{warmup}: {dt * 1000:.2f} ms", file=sys.stderr)

    walls: list[float] = []
    errors = 0
    for i in range(runs):
        t0 = time.perf_counter()
        try:
            fn()
            dt = time.perf_counter() - t0
            walls.append(dt)
            print(f"    run {i + 1}/{runs}: {dt * 1000:.2f} ms [ok]", file=sys.stderr)
        except Exception as exc:
            errors += 1
            print(f"    run {i + 1}/{runs}: FAIL — {exc}", file=sys.stderr)

    return BenchStats(name=name, walls=walls, input_bytes=input_bytes, errors=errors)


# ---------------------------------------------------------------------------
# Reporting
# ---------------------------------------------------------------------------

def _fmt_bytes(n: int | float) -> str:
    n = float(n)
    for unit, div in (("GiB", 1024**3), ("MiB", 1024**2), ("KiB", 1024), ("B", 1)):
        if n >= div or unit == "B":
            return f"{n / div:.2f} {unit}"
    return f"{n:.0f} B"


def _ratio(a: float, b: float) -> str:
    if b == 0 or a != a or b != b:
        return "n/a"
    return f"{a / b:.2f}x"


def print_report(rows: Sequence[BenchStats], mode: str) -> None:
    if not rows:
        return

    print()
    print("=" * 72)
    print("PYTHON JSON PARSE BENCHMARK")
    print("=" * 72)
    print(f"platform : {platform.system()} {platform.release()} ({platform.machine()})")
    print(f"python   : {sys.version.split()[0]}")
    print(f"mode     : {mode}")
    print(f"input    : {_fmt_bytes(rows[0].input_bytes)}")
    print()

    headers = ["parser", "median ms", "mean ms", "±stdev", "MB/s", "ok"]
    table: list[list[str]] = []
    for s in rows:
        ok = f"{len(s.walls)}/{len(s.walls) + s.errors}"
        table.append(
            [
                s.name,
                f"{s.median() * 1000:.2f}" if s.walls else "n/a",
                f"{s.mean() * 1000:.2f}" if s.walls else "n/a",
                f"{s.stdev() * 1000:.2f}" if s.walls else "n/a",
                f"{s.mb_per_s():.2f}" if s.walls else "n/a",
                ok,
            ]
        )

    widths = [max(len(h), *(len(r[i]) for r in table)) for i, h in enumerate(headers)]
    fmt = "  ".join(f"{{:<{w}}}" for w in widths)
    print(fmt.format(*headers))
    print(fmt.format(*("-" * w for w in widths)))
    for row in table:
        print(fmt.format(*row))

    baseline = next((s for s in rows if s.walls and s.name.startswith("json.")), None)
    if baseline is None:
        baseline = next((s for s in rows if s.walls), None)
    if baseline is not None:
        others = [s for s in rows if s is not baseline and s.walls]
        if others:
            print()
            print(f"relative to {baseline.name}:")
            for other in others:
                print(
                    f"  {other.name:<24}  wall {_ratio(other.median(), baseline.median())}"
                    f"  throughput {_ratio(other.mb_per_s(), baseline.mb_per_s())}"
                    "  (>1 wall = slower, >1 thr = faster)"
                )

    print()
    print("notes:")
    print("  - All parsers build a full Python object tree (dict/list/...).")
    print("  - MB/s uses median wall time over successful runs.")
    print("  - Result is discarded each run (only parse cost is measured).")
    print("  - orjson prefers bytes input; str path still accepted.")
    print("=" * 72)


# ---------------------------------------------------------------------------
# Structural equality (tolerate int↔float / float precision)
# ---------------------------------------------------------------------------

def _same_shape(a: Any, b: Any) -> bool:
    if isinstance(a, dict) and isinstance(b, dict):
        if a.keys() != b.keys():
            return False
        return all(_same_shape(a[k], b[k]) for k in a)
    if isinstance(a, list) and isinstance(b, list):
        if len(a) != len(b):
            return False
        return all(_same_shape(x, y) for x, y in zip(a, b))
    if isinstance(a, bool) and isinstance(b, bool):
        return a is b
    if isinstance(a, (int, float)) and isinstance(b, (int, float)) and not isinstance(
        a, bool
    ) and not isinstance(b, bool):
        return abs(float(a) - float(b)) <= 1e-6 * max(1.0, abs(float(a)))
    return a == b


# ---------------------------------------------------------------------------
# Import helpers
# ---------------------------------------------------------------------------

def import_rjson():
    try:
        import rjson  # type: ignore
        return rjson
    except ImportError as exc:
        raise SystemExit(
            "cannot import rjson (Python binding for rs-jq).\n"
            "Build/install it with:\n"
            "  pip install maturin\n"
            "  maturin develop --release\n"
            f"\nOriginal error: {exc}"
        ) from exc


def try_import(name: str, pip_hint: str) -> Any | None:
    try:
        return __import__(name)
    except ImportError:
        print(
            f"skip {name}: not installed (pip install {pip_hint})",
            file=sys.stderr,
        )
        return None


def parse_only(value: str) -> list[str]:
    """Parse --only: 'all', legacy 'both', or comma-separated parser names."""
    value = value.strip().lower()
    if value in ("all", "both"):
        # "both" kept for backward compat → all known parsers
        return list(PARSER_CHOICES)
    parts = [p.strip() for p in value.split(",") if p.strip()]
    if not parts:
        raise argparse.ArgumentTypeError("--only must name at least one parser")
    bad = [p for p in parts if p not in PARSER_CHOICES]
    if bad:
        raise argparse.ArgumentTypeError(
            f"unknown parser(s): {', '.join(bad)}; "
            f"choose from {', '.join(PARSER_CHOICES)}, all"
        )
    # preserve order, drop duplicates
    seen: set[str] = set()
    out: list[str] = []
    for p in parts:
        if p not in seen:
            seen.add(p)
            out.append(p)
    return out


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------

def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    p = argparse.ArgumentParser(
        description=(
            "Benchmark rjson (rs-jq PyO3) vs Python json / orjson / ujson."
        ),
    )
    p.add_argument(
        "--input",
        type=Path,
        help="JSON file to parse (default: generate under .bench/)",
    )
    p.add_argument(
        "--size-mb",
        type=float,
        default=5.0,
        help="fixture size in MiB when --input is omitted (default: 5)",
    )
    p.add_argument(
        "--force-generate",
        action="store_true",
        help="regenerate fixture even if it already exists",
    )
    p.add_argument("--warmup", type=int, default=1, help="warmup runs (default: 1)")
    p.add_argument("--runs", type=int, default=5, help="timed runs (default: 5)")
    p.add_argument(
        "--mode",
        choices=("loads", "load", "both"),
        default="both",
        help="loads=str, load=bytes (default: both)",
    )
    p.add_argument(
        "--only",
        type=parse_only,
        default=list(PARSER_CHOICES),
        metavar="PARSERS",
        help=(
            "parsers to run: all | both | comma-separated from "
            f"{','.join(PARSER_CHOICES)} (default: all)"
        ),
    )
    p.add_argument(
        "--verify",
        action="store_true",
        help="check that selected parsers produce equal Python object shapes once",
    )
    return p.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    if args.runs < 1:
        raise SystemExit("--runs must be >= 1")

    selected: list[str] = args.only

    if args.input:
        input_path = args.input.expanduser().resolve()
        if not input_path.is_file():
            raise SystemExit(f"input not found: {input_path}")
    else:
        input_path = ensure_fixture(
            ROOT / ".bench" / f"fixture_{args.size_mb:g}mb.json",
            args.size_mb,
            force=args.force_generate,
        )

    raw = input_path.read_bytes()
    text = raw.decode("utf-8")
    input_bytes = len(raw)
    print(
        f"input: {input_path} ({_fmt_bytes(input_bytes)})",
        file=sys.stderr,
    )

    modules: dict[str, Any] = {}
    if "json" in selected:
        modules["json"] = json
    if "rjson" in selected:
        modules["rjson"] = import_rjson()
    if "orjson" in selected:
        mod = try_import("orjson", "orjson")
        if mod is None:
            if selected == ["orjson"]:
                raise SystemExit("orjson is required for --only orjson")
            selected = [p for p in selected if p != "orjson"]
        else:
            modules["orjson"] = mod
    if "ujson" in selected:
        mod = try_import("ujson", "ujson")
        if mod is None:
            if selected == ["ujson"]:
                raise SystemExit("ujson is required for --only ujson")
            selected = [p for p in selected if p != "ujson"]
        else:
            modules["ujson"] = mod

    if not selected:
        raise SystemExit("no parsers available to benchmark")

    if args.verify:
        ref = json.loads(text)
        checks: list[tuple[str, Any]] = []
        if "rjson" in modules:
            checks.append(("rjson.rjson_loads", modules["rjson"].rjson_loads(text)))
            checks.append(("rjson.rjson_load", modules["rjson"].rjson_load(raw)))
        if "orjson" in modules:
            checks.append(("orjson.loads(str)", modules["orjson"].loads(text)))
            checks.append(("orjson.loads(bytes)", modules["orjson"].loads(raw)))
        if "ujson" in modules:
            checks.append(("ujson.loads(str)", modules["ujson"].loads(text)))
            checks.append(("ujson.loads(bytes)", modules["ujson"].loads(raw)))
        for label, value in checks:
            if not _same_shape(ref, value):
                raise SystemExit(f"verify failed: {label} shape != json.loads")
        print(
            f"verify: ok ({len(checks)} parsers; numbers may be float-only)",
            file=sys.stderr,
        )

    results: list[BenchStats] = []
    modes = ("loads", "load") if args.mode == "both" else (args.mode,)

    for mode in modes:
        for name in selected:
            if name == "json":
                if mode == "loads":
                    fn: Callable[[], Any] = lambda t=text: json.loads(t)
                    label = "json.loads"
                else:
                    fn = lambda b=raw: json.loads(b)
                    label = "json.loads(bytes)"
            elif name == "orjson":
                m = modules["orjson"]
                if mode == "loads":
                    fn = lambda t=text, mod=m: mod.loads(t)
                    label = "orjson.loads"
                else:
                    fn = lambda b=raw, mod=m: mod.loads(b)
                    label = "orjson.loads(bytes)"
            elif name == "ujson":
                m = modules["ujson"]
                if mode == "loads":
                    fn = lambda t=text, mod=m: mod.loads(t)
                    label = "ujson.loads"
                else:
                    fn = lambda b=raw, mod=m: mod.loads(b)
                    label = "ujson.loads(bytes)"
            elif name == "rjson":
                m = modules["rjson"]
                if mode == "loads":
                    fn = lambda t=text, mod=m: mod.rjson_loads(t)
                    label = "rjson.rjson_loads"
                else:
                    fn = lambda b=raw, mod=m: mod.rjson_load(b)
                    label = "rjson.rjson_load"
            else:
                continue

            results.append(
                bench(
                    label,
                    fn,
                    input_bytes=input_bytes,
                    warmup=args.warmup,
                    runs=args.runs,
                )
            )

    print_report(results, args.mode)

    if any(s.errors and not s.walls for s in results):
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
