# Verification notes

Local verification on 2026-10-20 used macOS ARM64, Rust/Cargo 1.99.1, Git, Bash 3.2, Zsh 5.9, and Python 3.9. Fish and PowerShell are unavailable locally.

Executed here:

- `cargo fmt --all -- --check` and `cargo clippy --locked --offline --all-targets -- -D warnings`.
- `cargo test --locked --offline`: eleven unit tests and seventeen temporary-repository integration tests passed.
- `cargo build --release --locked --offline`.
- Bash and Zsh integration syntax checks.
- Fourteen real Readline/ZLE Tab insertion tests against the compiled Rust binary, including double-quoted insertion, shell operators, multiple suggestions, empty Zsh quotes, written messages before/after the cursor, idempotent loading, and delegation to both test and installed Git completion functions. Bash 3.2's unsafe default-fallback setup is refused without replacing Git's completion.
- Python archive self-checks for contents, executable permissions, target validation, and SHA-256 sidecars.
- Native macOS ARM64 archive installation in a temporary directory, including PATH lookup, embedded shell scripts, suggestions, replacement, uninstall, and index/worktree preservation.
- Cargo source installation, forced replacement, and uninstall using an isolated install root.
- Git command checks in a real unborn repository with spaces, a newline, and pathspec metacharacters in paths, executable helpers configured, and an unchanged index/worktree.
- CI/release workflow and issue-template YAML syntax checks.
- Release-build latency measurements below.
- A replay of the reported multi-file commit in a disposable repository produced a summary of AI reply generation, push notification job processing, and push subscription persistence. The original repository was only read.

The unusual-filename integration fixture inserts its non-UTF-8 path directly into Git's index. This tests byte preservation even on filesystems that reject creating that filename.

`python3 scripts/benchmark.py` measured thirty warm requests per disposable repository:

| Staged files | Median | p95 |
| --- | --- | --- |
| 1 | 30.0 ms | 39.7 ms |
| 8 | 44.0 ms | 54.3 ms |
| 2,000 | 23.8 ms | 28.6 ms |

These are local fixture measurements, not a general guarantee. The one- and eight-file cases include patch analysis; the 2,000-file case uses a bounded path summary. Four concurrent patch reads reduced the eight-file median from 93.4 ms in the immediately preceding sequential run to 44.0 ms. Patch results are consumed in file order, preserving deterministic suggestions and the combined analysis budget.

Not executed here:

- Native Fish or PowerShell completion tests.
- Linux/Windows/Intel execution, Rust 1.74 compatibility, or execution of cross-platform release archives.
- Upgrading from an older published release; replacement tests use the current build.

These checks are implemented in the CI and release workflows. Their presence is not evidence that they have passed. The release workflow now waits for the full CI workflow and checks each native archive before upload.

The GitHub destination is [memar1992/git-autocomplete](https://github.com/memar1992/git-autocomplete). The checkout now has an initial commit, but remote CI and publishing could not run here: writing `.git/config` is denied by the workspace permissions, GitHub CLI and API credentials are unavailable, and terminal DNS access to GitHub fails. The remote repository's contents and CI status could not be inspected. Fish and PowerShell are not installed, and access to the local Docker daemon is denied. A macOS ARM64 candidate archive and checksum were produced locally under `dist/`. No GitHub Release, crates.io package, Homebrew formula, Scoop manifest, or Winget package was published by this session.
