# Contributing

Use Rust 1.74+ and Git 2.30+. The core has no crate dependencies. Python 3.9+ runs the archive and Unix terminal checks; Fish and PowerShell checks use their respective shells.

## Submit a pull request

1. Fork the repository and create a branch for one focused change.
2. Add or update tests for behavior changes. For detection rules, include a small staged-diff example and a counterexample.
3. Run the local checks below for the parts you changed.
4. Open a pull request against `main`. Complete the PR template with commands, results, operating system, Rust version, and shells tested. Explain skipped checks.
5. A maintainer reviews the code and validation before merging. Address review comments in the same PR.

Pushes and pull requests do not run CI. Contributors report local results, and maintainers reproduce relevant checks during review. No hosted runner or paid account is required to submit a contribution.

Maintainers should require pull requests and review for `main`. Remove any required CI status checks left from the previous setup so PRs do not wait for jobs that no longer exist.

## Local checks

```sh
cargo fmt --all -- --check
cargo clippy --locked --offline --all-targets -- -D warnings
cargo test --locked --offline
cargo build --locked --offline
```

For shell or packaging changes, also run the checks available on your machine. On macOS or Linux:

```sh
python3 scripts/test-shells.py
python3 scripts/test-package.py
# Run when Fish is installed:
PATH="$PWD/target/debug:$PATH" fish --no-config scripts/test-fish.fish
```

On Windows, run `scripts/test-powershell.ps1` from PowerShell 7.3+ with `target/debug` on PATH, and `python scripts/test-package.py` for packaging changes. `python3 scripts/test-shells.py --stub` checks only Bash/Zsh adapters and does not validate the Rust engine. Do not label a skipped platform test as passing.

The Rust integration tests create real temporary repositories, stage fixture changes, and check the index and worktree before and after suggestion generation. Unix terminal checks press Tab in real Bash and Zsh sessions. A test-local `git` shell function records the completed argument array so tests never execute a real commit through the completed line. Fish and PowerShell tests call their native completion APIs and check that existing completions remain available.

## Add a detection rule

1. Put path categories in `src/analyzer/files.rs` or language patterns in `src/analyzer/patterns.rs`.
2. Add a description in `src/generator/mod.rs`. Use a fixed confidence tier with a reason that names the actual evidence.
3. Add a small fixture and temporary-repository test in `tests/`. Include a counterexample where the rule must fall back.
4. Document the rule's limits. A function named `fixCrash` or a comment saying "faster" does not prove a bug fix or performance gain.

Keep results deterministic. Avoid filesystem traversal, executing repository code, new network behavior, and guessing intent from branch names. Do not add runtime configuration, daemons, or AST dependencies without a measured need.

## Release

Update Cargo.toml, Cargo.lock, and CHANGELOG.md together in a release PR. Record local checks, including native shell checks and Rust 1.74 compatibility where available. Review the changes and any unverified platforms before merging, then tag the reviewed commit as `vX.Y.Z`.

The release workflow runs only for version tags. It builds the six supported targets, validates the tag, checks each archive through an isolated install, produces SHA-256 checksums, and creates a draft GitHub Release. It does not run a separate CI workflow or replace the local checks reported in the release PR. Review the draft and build results before publishing it. The workflow does not publish to crates.io or any package manager.

To test a local archive, build and package for your own platform. For example, on an Apple Silicon Mac:

```sh
cargo build --release --locked --offline
python3 scripts/package.py --binary target/release/git-autocomplete --target aarch64-apple-darwin
python3 scripts/test-install.py dist/git-autocomplete-aarch64-apple-darwin.tar.gz
```

Use the matching target from the README on other platforms, with `.exe` and `.zip` on Windows. The install check verifies the checksum, PATH lookup, embedded integrations, CLI behavior, index/worktree preservation, replacement with the current build, and uninstall inside a temporary directory. It does not change your installed tool or shell profile. Upgrading from an older published version will need a separate check once one exists.

Release packaging still uses GitHub Actions and requires an account able to start hosted jobs. If hosted builds are unavailable, maintainers can build and test archives locally using the commands above and record each archive's target and verification results in the release PR.

The release workflow uses GitHub's [documented native runner labels](https://docs.github.com/en/actions/reference/runners/github-hosted-runners). Keep the matrix and installation table in sync when runner support changes. Third-party Actions run only during release builds; the installed tool works offline.
