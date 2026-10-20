#!/usr/bin/env python3
"""Assert archive contents, executable modes, checksums, and target validation."""
import hashlib
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import zipfile

sys.dont_write_bytecode = True
from package import package


with tempfile.TemporaryDirectory(prefix="git-autocomplete-package-") as temp:
    root = Path(temp)
    binary = root / "binary"
    binary.write_bytes(b"executable fixture\n")
    for target in ("aarch64-apple-darwin", "x86_64-pc-windows-msvc"):
        archive = package(binary, target, root / "dist")
        expected = hashlib.sha256(archive.read_bytes()).hexdigest()
        assert archive.with_name(archive.name + ".sha256").read_bytes() == f"{expected}  {archive.name}\n".encode("ascii")
        folder = "git-autocomplete-" + target + "/"
        if archive.suffix == ".zip":
            with zipfile.ZipFile(archive) as handle:
                assert handle.read(folder + "git-autocomplete.exe") == binary.read_bytes()
                names = handle.namelist()
        else:
            with tarfile.open(archive) as handle:
                member = handle.getmember(folder + "git-autocomplete")
                assert member.mode & 0o111 == 0o111
                assert handle.extractfile(member).read() == binary.read_bytes()
                names = handle.getnames()
        for suffix in ("README.md", "LICENSE", "CHANGELOG.md", "completions/git-autocomplete.ps1", "completions/git-autocomplete.zsh"):
            assert folder + suffix in names
        archive.with_name(archive.name + ".sha256").write_bytes(b"tampered\n")
        rejected = subprocess.run([sys.executable, str(Path(__file__).with_name("test-install.py")), str(archive)], capture_output=True, text=True)
        assert rejected.returncode != 0 and "checksum mismatch" in rejected.stderr
    try:
        package(binary, "../../invalid", root / "dist")
    except ValueError:
        pass
    else:
        raise AssertionError("unsafe target accepted")
print("Archive contents, modes, and checksums passed")
