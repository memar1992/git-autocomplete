# Contributing

Use Rust 1.74+ and Git 2.30+. The core has no crate dependencies. Python 3.9+ runs the archive and Unix terminal checks; Fish and PowerShell checks use their respective shells.

```sh
cargo fmt --all
cargo clippy --locked --offline --all-targets -- -D warnings
cargo test --locked --offline
cargo build --locked --offline
python3 scripts/test-shells.py
python3 scripts/test-package.py
PATH="$PWD/target/debug:$PATH" fish --no-config scripts/test-fish.fish
```

On Windows, run `scripts/test-powershell.ps1` from PowerShell 7.3+ with `target/debug` on PATH. `python3 scripts/test-shells.py --stub` checks only Bash/Zsh adapters and does not validate the Rust engine. CI runs the real binary and builds on each supported architecture. Do not label a skipped platform test as passing.

The Rust integration tests create real temporary repositories, stage fixture changes, and check the index and worktree before and after suggestion generation. Unix terminal checks press Tab in real Bash and Zsh sessions. A test-local `git` shell function records the completed argument array so tests never execute a real commit through the completed line. Fish and PowerShell tests call their native completion APIs and check that existing completions remain available.

## Add a detection rule

1. Put path categories in `src/analyzer/files.rs` or language patterns in `src/analyzer/patterns.rs`.
2. Add a description in `src/generator/mod.rs`. Use a fixed confidence tier with a reason that names the actual evidence.
3. Add a small fixture and temporary-repository test in `tests/`. Include a counterexample where the rule must fall back.
4. Document the rule's limits. A function named `fixCrash` or a comment saying "faster" does not prove a bug fix or performance gain.

Keep results deterministic. Avoid filesystem traversal, executing repository code, new network behavior, and guessing intent from branch names. Do not add runtime configuration, daemons, or AST dependencies without a measured need.

## Release

Update Cargo.toml, Cargo.lock, and CHANGELOG.md together. Run all checks, including the native shell checks, then tag the version as `vX.Y.Z`. A release first runs the reusable CI workflow, including all six native targets, Fish, PowerShell, and Rust 1.74. Release builds wait for those checks to pass. Each archive then passes an isolated install check before upload. The workflow validates the tag, produces archives and SHA-256 checksums, and creates a draft GitHub Release. Review the draft and its CI results before publishing it. The workflow does not publish to crates.io or any package manager.

To test a local archive, build and package for your own platform. For example, on an Apple Silicon Mac:

```sh
cargo build --release --locked --offline
python3 scripts/package.py --binary target/release/git-autocomplete --target aarch64-apple-darwin
python3 scripts/test-install.py dist/git-autocomplete-aarch64-apple-darwin.tar.gz
```

Use the matching target from the README on other platforms, with `.exe` and `.zip` on Windows. The install check verifies the checksum, PATH lookup, embedded integrations, CLI behavior, index/worktree preservation, replacement with the current build, and uninstall inside a temporary directory. It does not change your installed tool or shell profile. Upgrading from an older published version will need a separate check once one exists.

CI also supports [manual runs from the Actions tab](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/manually-run-a-workflow) after the workflow reaches the default branch. The release workflow [reuses CI at the same commit](https://docs.github.com/en/actions/how-tos/reuse-automations/reuse-workflows) so a tag cannot bypass the shell and platform checks.

The workflow uses GitHub's [documented native runner labels](https://docs.github.com/en/actions/reference/runners/github-hosted-runners). Keep the matrix and installation table in sync when runner support changes. All third-party Actions run only during development and release builds; the installed tool works offline.
