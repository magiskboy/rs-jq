#!/usr/bin/env python3
"""Benchmark rs-jq CLI JSON parse throughput against stock jq.

Reports wall time, tokens/s, MB/s, peak RSS, and a side-by-side comparison.

Examples:
  python3 scripts/bench_parse.py
  python3 scripts/bench_parse.py --size-mb 10 --runs 5
  python3 scripts/bench_parse.py --input path/to/data.json --script '.'
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import re
import shutil
import statistics
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Iterable, Sequence


ROOT = Path(__file__).resolve().parents[1]


# ---------------------------------------------------------------------------
# Token counter (JSON tokens, whitespace skipped — same idea as rs-jq lexer)
# ---------------------------------------------------------------------------

_TOKEN_RE = re.compile(
    r"""
    (?P<string>"(?:\\.|[^"\\])*")
  | (?P<number>-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?)
  | (?P<literal>true|false|null)
  | (?P<punct>[{}\[\]:,])
  | (?P<ws>\s+)
  | (?P<invalid>.)
    """,
    re.VERBOSE,
)


def count_json_tokens(text: str) -> int:
    """Count non-whitespace JSON tokens in *text*."""
    n = 0
    for m in _TOKEN_RE.finditer(text):
        if m.lastgroup in ("ws", "invalid"):
            if m.lastgroup == "invalid":
                raise ValueError(f"invalid JSON token near offset {m.start()}")
            continue
        n += 1
    return n


# ---------------------------------------------------------------------------
# Fixture generation
# ---------------------------------------------------------------------------

def generate_json(target_bytes: int, seed: int = 1) -> str:
    """Build a compact JSON array of objects until size >= target_bytes."""
    items: list[dict] = []
    i = seed
    # Grow in chunks; check size periodically to avoid huge Python overhead.
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
    text = generate_json(target)
    path.write_text(text, encoding="utf-8")
    return path


# ---------------------------------------------------------------------------
# Process measurement
# ---------------------------------------------------------------------------

@dataclass(frozen=True)
class RunResult:
    wall_s: float
    user_s: float | None
    sys_s: float | None
    peak_rss_bytes: int | None
    exit_code: int
    stderr: str


def _parse_time_l(stderr: str) -> tuple[float | None, float | None, int | None]:
    """Parse macOS `/usr/bin/time -l` output."""
    user = sys_ = rss = None
    # "        0.01 real         0.00 user         0.00 sys"
    m = re.search(
        r"([\d.]+)\s+real\s+([\d.]+)\s+user\s+([\d.]+)\s+sys",
        stderr,
    )
    if m:
        user = float(m.group(2))
        sys_ = float(m.group(3))
    m = re.search(r"(\d+)\s+maximum resident set size", stderr)
    if m:
        rss = int(m.group(1))
    return user, sys_, rss


def _parse_time_v(stderr: str) -> tuple[float | None, float | None, int | None]:
    """Parse GNU `/usr/bin/time -v` output."""
    user = sys_ = rss = None
    m = re.search(r"User time \(seconds\):\s*([\d.]+)", stderr)
    if m:
        user = float(m.group(1))
    m = re.search(r"System time \(seconds\):\s*([\d.]+)", stderr)
    if m:
        sys_ = float(m.group(1))
    m = re.search(r"Maximum resident set size \(kbytes\):\s*(\d+)", stderr)
    if m:
        # GNU time reports KiB
        rss = int(m.group(1)) * 1024
    return user, sys_, rss


def _time_wrapper() -> tuple[list[str], Callable[[str], tuple[float | None, float | None, int | None]]] | None:
    time_bin = shutil.which("time")
    # Prefer the real binary; shell builtin has no -l/-v.
    candidates = ["/usr/bin/time", time_bin]
    system = platform.system()
    for candidate in candidates:
        if not candidate or not Path(candidate).exists():
            continue
        if system == "Darwin":
            return [candidate, "-l"], _parse_time_l
        # Linux / others with GNU time
        return [candidate, "-v"], _parse_time_v
    return None


def run_once(cmd: Sequence[str], discard_stdout: bool = True) -> RunResult:
    wrapper = _time_wrapper()
    stdout_dest = subprocess.DEVNULL if discard_stdout else None

    if wrapper is None:
        t0 = time.perf_counter()
        proc = subprocess.run(
            list(cmd),
            stdout=stdout_dest,
            stderr=subprocess.PIPE,
            text=True,
        )
        wall = time.perf_counter() - t0
        return RunResult(
            wall_s=wall,
            user_s=None,
            sys_s=None,
            peak_rss_bytes=None,
            exit_code=proc.returncode,
            stderr=proc.stderr or "",
        )

    prefix, parse = wrapper
    full = prefix + list(cmd)
    t0 = time.perf_counter()
    proc = subprocess.run(
        full,
        stdout=stdout_dest,
        stderr=subprocess.PIPE,
        text=True,
    )
    wall = time.perf_counter() - t0
    user, sys_, rss = parse(proc.stderr or "")
    return RunResult(
        wall_s=wall,
        user_s=user,
        sys_s=sys_,
        peak_rss_bytes=rss,
        exit_code=proc.returncode,
        stderr=proc.stderr or "",
    )


# ---------------------------------------------------------------------------
# Benchmark aggregation
# ---------------------------------------------------------------------------

@dataclass
class BenchStats:
    name: str
    cmd: list[str]
    runs: list[RunResult]
    input_bytes: int
    token_count: int

    @property
    def ok_runs(self) -> list[RunResult]:
        return [r for r in self.runs if r.exit_code == 0]

    def _walls(self) -> list[float]:
        return [r.wall_s for r in self.ok_runs]

    def mean_wall(self) -> float:
        walls = self._walls()
        return statistics.mean(walls) if walls else float("nan")

    def median_wall(self) -> float:
        walls = self._walls()
        return statistics.median(walls) if walls else float("nan")

    def stdev_wall(self) -> float:
        walls = self._walls()
        return statistics.stdev(walls) if len(walls) >= 2 else 0.0

    def mean_rss(self) -> float | None:
        vals = [r.peak_rss_bytes for r in self.ok_runs if r.peak_rss_bytes is not None]
        return statistics.mean(vals) if vals else None

    def max_rss(self) -> int | None:
        vals = [r.peak_rss_bytes for r in self.ok_runs if r.peak_rss_bytes is not None]
        return max(vals) if vals else None

    def tokens_per_s(self) -> float:
        t = self.median_wall()
        return self.token_count / t if t > 0 else float("nan")

    def mb_per_s(self) -> float:
        t = self.median_wall()
        mb = self.input_bytes / (1024 * 1024)
        return mb / t if t > 0 else float("nan")


def benchmark(
    name: str,
    cmd: list[str],
    *,
    input_bytes: int,
    token_count: int,
    warmup: int,
    runs: int,
) -> BenchStats:
    print(f"\n==> {name}", file=sys.stderr)
    print(f"    cmd: {' '.join(cmd)}", file=sys.stderr)

    for i in range(warmup):
        r = run_once(cmd)
        if r.exit_code != 0:
            print(
                f"    warmup {i + 1} failed (exit {r.exit_code}):\n{r.stderr}",
                file=sys.stderr,
            )
            break
        print(f"    warmup {i + 1}/{warmup}: {r.wall_s * 1000:.2f} ms", file=sys.stderr)

    results: list[RunResult] = []
    for i in range(runs):
        r = run_once(cmd)
        results.append(r)
        rss = f", rss={_fmt_bytes(r.peak_rss_bytes)}" if r.peak_rss_bytes else ""
        status = "ok" if r.exit_code == 0 else f"FAIL({r.exit_code})"
        print(
            f"    run {i + 1}/{runs}: {r.wall_s * 1000:.2f} ms [{status}]{rss}",
            file=sys.stderr,
        )
        if r.exit_code != 0:
            print(r.stderr, file=sys.stderr)

    return BenchStats(
        name=name,
        cmd=cmd,
        runs=results,
        input_bytes=input_bytes,
        token_count=token_count,
    )


# ---------------------------------------------------------------------------
# Reporting
# ---------------------------------------------------------------------------

def _fmt_bytes(n: int | float | None) -> str:
    if n is None:
        return "n/a"
    n = float(n)
    for unit, div in (("GiB", 1024**3), ("MiB", 1024**2), ("KiB", 1024), ("B", 1)):
        if n >= div or unit == "B":
            return f"{n / div:.2f} {unit}"
    return f"{n:.0f} B"


def _fmt_num(n: float | None, suffix: str = "") -> str:
    if n is None or n != n:  # NaN
        return "n/a"
    if abs(n) >= 1_000_000:
        return f"{n / 1_000_000:.2f}M{suffix}"
    if abs(n) >= 1_000:
        return f"{n / 1_000:.2f}k{suffix}"
    return f"{n:.2f}{suffix}"


def _ratio(a: float, b: float) -> str:
    if b == 0 or a != a or b != b:
        return "n/a"
    return f"{a / b:.2f}x"


def print_report(stats: Iterable[BenchStats], script: str) -> None:
    rows = list(stats)
    if not rows:
        return

    print()
    print("=" * 72)
    print("PARSE BENCHMARK REPORT")
    print("=" * 72)
    print(f"platform : {platform.system()} {platform.release()} ({platform.machine()})")
    print(f"script   : {script!r}")
    print(f"input    : {_fmt_bytes(rows[0].input_bytes)}  ({rows[0].token_count:,} tokens)")
    print()

    headers = [
        "tool",
        "median ms",
        "mean ms",
        "±stdev",
        "MB/s",
        "tokens/s",
        "peak RSS",
        "mean RSS",
        "ok",
    ]
    table: list[list[str]] = []
    for s in rows:
        table.append(
            [
                s.name,
                f"{s.median_wall() * 1000:.2f}",
                f"{s.mean_wall() * 1000:.2f}",
                f"{s.stdev_wall() * 1000:.2f}",
                f"{s.mb_per_s():.2f}",
                _fmt_num(s.tokens_per_s()),
                _fmt_bytes(s.max_rss()),
                _fmt_bytes(s.mean_rss()),
                f"{len(s.ok_runs)}/{len(s.runs)}",
            ]
        )

    widths = [max(len(h), *(len(r[i]) for r in table)) for i, h in enumerate(headers)]
    fmt = "  ".join(f"{{:<{w}}}" for w in widths)
    print(fmt.format(*headers))
    print(fmt.format(*("-" * w for w in widths)))
    for row in table:
        print(fmt.format(*row))

    if len(rows) >= 2:
        base, other = rows[0], rows[1]
        print()
        print(f"relative to {base.name}:")
        print(f"  wall time  : {_ratio(other.median_wall(), base.median_wall())}  (>1 = slower)")
        print(f"  throughput : {_ratio(other.mb_per_s(), base.mb_per_s())}  (>1 = faster)")
        if base.max_rss() and other.max_rss():
            print(
                f"  peak RSS   : {_ratio(float(other.max_rss()), float(base.max_rss()))}"
                "  (>1 = more memory)"
            )

    print()
    print("notes:")
    print("  - MB/s and tokens/s use median wall time over successful runs.")
    print("  - tokens = non-whitespace JSON tokens (string/number/literal/punct).")
    print("  - peak RSS from `/usr/bin/time` (macOS -l / GNU -v); includes process overhead.")
    print("  - stdout discarded so I/O of printed JSON does not dominate.")
    print("=" * 72)


# ---------------------------------------------------------------------------
# CLI discovery
# ---------------------------------------------------------------------------

def resolve_rs_jq(explicit: str | None) -> Path:
    if explicit:
        p = Path(explicit).expanduser().resolve()
        if not p.is_file():
            raise SystemExit(f"rs-jq binary not found: {p}")
        return p
    release = ROOT / "target" / "release" / "jq"
    debug = ROOT / "target" / "debug" / "jq"
    if release.is_file():
        return release
    if debug.is_file():
        print("warning: using debug build (prefer: cargo build --features cli --release)", file=sys.stderr)
        return debug
    raise SystemExit(
        "rs-jq binary not found. Build with:\n"
        "  cargo build --features cli --release"
    )


def resolve_stock_jq(explicit: str | None) -> Path:
    if explicit:
        p = Path(explicit).expanduser().resolve()
        if not p.is_file():
            raise SystemExit(f"jq binary not found: {p}")
        return p
    found = shutil.which("jq")
    if not found:
        raise SystemExit("stock jq not found on PATH (brew install jq)")
    return Path(found)


def rs_jq_cmd(binary: Path, input_path: Path, script: str) -> list[str]:
    return [str(binary), "--input", str(input_path), "--script", script]


def stock_jq_cmd(binary: Path, input_path: Path, script: str) -> list[str]:
    return [str(binary), script, str(input_path)]


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------

def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    p = argparse.ArgumentParser(
        description="Benchmark rs-jq CLI parse performance vs stock jq.",
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
    p.add_argument("--script", default=".", help="jq/jql script (default: '.')")
    p.add_argument("--warmup", type=int, default=1, help="warmup runs (default: 1)")
    p.add_argument("--runs", type=int, default=5, help="timed runs (default: 5)")
    p.add_argument("--rs-jq", dest="rs_jq", help="path to rs-jq binary")
    p.add_argument("--jq", dest="jq", help="path to stock jq binary")
    p.add_argument(
        "--only",
        choices=("rs-jq", "jq", "both"),
        default="both",
        help="which tools to benchmark (default: both)",
    )
    return p.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    if args.runs < 1:
        raise SystemExit("--runs must be >= 1")

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
    input_bytes = len(raw)
    text = raw.decode("utf-8")
    try:
        token_count = count_json_tokens(text)
    except ValueError as exc:
        raise SystemExit(f"failed to tokenize input: {exc}") from exc

    print(
        f"input: {input_path} ({_fmt_bytes(input_bytes)}, {token_count:,} tokens)",
        file=sys.stderr,
    )

    results: list[BenchStats] = []

    if args.only in ("rs-jq", "both"):
        rs_bin = resolve_rs_jq(args.rs_jq)
        results.append(
            benchmark(
                "rs-jq",
                rs_jq_cmd(rs_bin, input_path, args.script),
                input_bytes=input_bytes,
                token_count=token_count,
                warmup=args.warmup,
                runs=args.runs,
            )
        )

    if args.only in ("jq", "both"):
        jq_bin = resolve_stock_jq(args.jq)
        results.append(
            benchmark(
                "jq",
                stock_jq_cmd(jq_bin, input_path, args.script),
                input_bytes=input_bytes,
                token_count=token_count,
                warmup=args.warmup,
                runs=args.runs,
            )
        )

    print_report(results, args.script)

    if any(len(s.ok_runs) == 0 for s in results):
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
