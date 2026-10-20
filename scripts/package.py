#!/usr/bin/env python3
"""Bundle the executable, integrations, and docs; write a SHA-256 sidecar."""
import argparse
import hashlib
from pathlib import Path
import shutil
import tarfile
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def package(binary, target, output):
    if not binary.is_file():
        raise ValueError(f"missing binary: {binary}")
    if not target or any(not (c.isascii() and (c.isalnum() or c in '-_')) for c in target):
        raise ValueError("invalid Rust target")
    output.mkdir(parents=True, exist_ok=True)
    name = f"git-autocomplete-{target}"
    windows = "windows" in target
    archive = output / (name + (".zip" if windows else ".tar.gz"))
    with tempfile.TemporaryDirectory() as tmp:
        folder = Path(tmp) / name
        folder.mkdir()
        shutil.copy2(binary, folder / ("git-autocomplete.exe" if windows else "git-autocomplete"))
        if not windows:
            (folder / "git-autocomplete").chmod(0o755)
        shutil.copytree(ROOT / "completions", folder / "completions")
        for doc in ("README.md", "LICENSE", "CHANGELOG.md", "CONTRIBUTING.md", "VERIFICATION.md"):
            shutil.copy2(ROOT / doc, folder / doc)
        (folder / "scripts").mkdir()
        shutil.copy2(ROOT / "scripts/DEMO.md", folder / "scripts/DEMO.md")
        if windows:
            with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as handle:
                for item in sorted(folder.rglob("*")):
                    if item.is_file():
                        handle.write(item, item.relative_to(folder.parent))
        else:
            with tarfile.open(archive, "w:gz") as handle:
                handle.add(folder, arcname=name)
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    with archive.with_name(archive.name + ".sha256").open("w", encoding="ascii", newline="\n") as checksum:
        checksum.write(f"{digest}  {archive.name}\n")
    return archive


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--target", required=True)
    parser.add_argument("--output", type=Path, default=ROOT / "dist")
    args = parser.parse_args()
    print(package(args.binary, args.target, args.output))
