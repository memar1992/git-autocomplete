#!/usr/bin/env python3
"""Measure end-to-end warm suggest latency in disposable small and large repositories."""
import argparse
import os
from pathlib import Path
import statistics
import subprocess
import tempfile
import time


def measure(binary, count, runs):
    with tempfile.TemporaryDirectory(prefix="git-autocomplete-bench-") as tmp:
        root = Path(tmp)
        subprocess.run(["git", "init", "-q", tmp], check=True)
        source = root / "src" / "auth"
        source.mkdir(parents=True)
        for i in range(count):
            (source / f"module_{i}.rs").write_text("pub fn validate() -> bool { true }\n" * (1 if count == 1 else 100))
        subprocess.run(["git", "-C", tmp, "add", "--all"], check=True)
        command = [str(binary), "suggest", "--format", "json"]
        for _ in range(3):
            subprocess.run(command, cwd=tmp, check=True, stdout=subprocess.DEVNULL)
        samples = []
        for _ in range(runs):
            start = time.perf_counter()
            subprocess.run(command, cwd=tmp, check=True, stdout=subprocess.DEVNULL)
            samples.append((time.perf_counter() - start) * 1000)
        samples.sort()
        print(f"{count:5d} staged files: median {statistics.median(samples):.1f} ms, p95 {samples[min(len(samples)-1, int(len(samples)*0.95))]:.1f} ms ({runs} runs)")
        assert len(samples) == runs and all(value > 0 for value in samples)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("target/release/git-autocomplete" + (".exe" if os.name == "nt" else "")))
    parser.add_argument("--runs", type=int, default=30)
    args = parser.parse_args()
    if args.runs < 1:
        parser.error("--runs must be positive")
    for files in (1, 8, 2000):
        measure(args.binary.resolve(), files, args.runs)
