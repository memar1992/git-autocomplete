# git-autocomplete

Offline commit message suggestions from staged Git changes. Keep using Git:

```sh
git add src/auth/login.ts tests/auth/login.test.ts
git commit -m <TAB>
```

An added exported function might suggest:

```text
feat(auth): add validateLogin function
chore(auth): add login implementation
```

Tab inserts a shell-escaped message as one argument. Enter runs your normal Git command. The tool never stages, commits, or writes repository files. Written messages are left alone.

The Rust executable calls the installed Git CLI and applies fixed rules to paths and a bounded staged diff. There are no runtime dependencies beyond Git, no model, no API key, no telemetry, and no network access during suggestions. No project configuration is needed.

## Install

Install once for your user account, then activate completion for your shell. The executable works in every Git repository without per-project setup. You do not need to keep a terminal open in this project's directory.

This source tree includes a release workflow. Hosted binaries and Cargo, Homebrew, Scoop, or Winget packages have **not been published** by this project setup. Until a maintainer publishes a release, install from source:

```sh
# Run from this project's directory, which contains Cargo.toml.
# Rust 1.74+ and Git 2.30+ must already be installed.
cargo install --path . --locked --offline
```

Cargo installs into `~/.cargo/bin`, or `%USERPROFILE%\.cargo\bin` on Windows. Building this package needs no crates downloaded from the network.

On macOS or Linux, make the executable available in your current terminal and verify the installation:

```sh
export PATH="$HOME/.cargo/bin:$PATH"
git-autocomplete version
```

Keep that PATH addition in your shell startup file so it also works in new terminals. The Zsh setup below includes it. On Windows, add `%USERPROFILE%\.cargo\bin` to your user PATH if needed, open a new terminal, and run `git-autocomplete version`.

Source builds also need a platform linker: Xcode Command Line Tools on macOS, a C compiler toolchain on Linux, or Visual Studio C++ Build Tools on Windows. Release archives do not require Rust or build tools.

### Release archives

When a release is available on this repository's Releases page, download the matching archive and `SHA256SUMS`. Each archive includes the executable, the four shell integrations, and this documentation. The release workflow builds:

| System | Rust target | Archive |
| --- | --- | --- |
| macOS Apple Silicon | aarch64-apple-darwin | .tar.gz |
| macOS Intel | x86_64-apple-darwin | .tar.gz |
| Linux x86_64 | x86_64-unknown-linux-gnu | .tar.gz |
| Linux ARM64 | aarch64-unknown-linux-gnu | .tar.gz |
| Windows x86_64 | x86_64-pc-windows-msvc | .zip |
| Windows ARM64 | aarch64-pc-windows-msvc | .zip |

Linux archives use glibc 2.35 or later. Alpine/musl builds are not included. macOS and Windows binaries are unsigned and are not notarized. ARM64 runner availability depends on the hosting repository's GitHub Actions plan.

On macOS or Linux, substitute your target in the following commands:

```sh
# macOS uses shasum -a 256; Linux can use sha256sum.
shasum -a 256 git-autocomplete-aarch64-apple-darwin.tar.gz
# Compare the result with the matching line in SHA256SUMS before extracting.
tar -xzf git-autocomplete-aarch64-apple-darwin.tar.gz
mkdir -p "$HOME/.local/bin"
install -m 755 git-autocomplete-aarch64-apple-darwin/git-autocomplete "$HOME/.local/bin/git-autocomplete"
export PATH="$HOME/.local/bin:$PATH"
git-autocomplete version
```

Keep that PATH addition in your shell profile if needed. On Windows, run these PowerShell commands after downloading the x86_64 archive:

```powershell
Get-FileHash .\git-autocomplete-x86_64-pc-windows-msvc.zip -Algorithm SHA256
# Compare with SHA256SUMS before extracting.
Expand-Archive .\git-autocomplete-x86_64-pc-windows-msvc.zip -DestinationPath .\git-autocomplete-release
$install = "$env:LOCALAPPDATA\git-autocomplete"
New-Item -ItemType Directory -Force $install | Out-Null
Copy-Item .\git-autocomplete-release\git-autocomplete-x86_64-pc-windows-msvc\git-autocomplete.exe $install
```

Add `%LOCALAPPDATA%\git-autocomplete` to your user PATH through Windows Environment Variables, open a new terminal, and run `git-autocomplete version`. Install Git for Windows separately if `git version` does not work. Git Bash uses the Bash setup below.

## Activate completion

Run the setup for your shell once, then add only the indicated source line to its startup file. These commands create an integration file; they do not edit your startup files. The scripts are also available in `completions/` and inside release archives.

### Zsh

```zsh
mkdir -p "$HOME/.config/git-autocomplete"
git-autocomplete shell zsh > "$HOME/.config/git-autocomplete/init.zsh"
```

Add these lines to `~/.zshrc`, after `compinit` and any Git completion plugins or Oh My Zsh setup. The PATH line is for Cargo installs; for an archive install, use its installation directory instead.

```zsh
export PATH="$HOME/.cargo/bin:$PATH"
source "$HOME/.config/git-autocomplete/init.zsh"
```

If your shell setup does not already initialize completion, add this immediately before the `source` line:

```zsh
autoload -Uz compinit && compinit
```

Open a new terminal to load the setup. Completion is now available in every Git repository for your user account.

The dispatcher retains your existing Git completion function. At a bare message argument, it inserts a double-quoted message, such as `git commit -m "chore(auth): update login validation"`. Zsh escapes special characters inside that string. Completion inside existing empty quotes preserves their quoting context. Multiple suggestions follow your existing Zsh menu settings. For a selectable menu, you can enable `zstyle ':completion:*' menu select`.

### Bash

Load your distribution's Git Bash completion first, then run:

```bash
mkdir -p "$HOME/.config/git-autocomplete"
git-autocomplete shell bash > "$HOME/.config/git-autocomplete/init.bash"
```

Add to `~/.bashrc`, after Git's completion setup:

```bash
source "$HOME/.config/git-autocomplete/init.bash"
```

Use Bash 4.4+ for a bare empty argument, `git commit -m ` or `git commit --message `. Empty quotes and attached forms such as `--message=` are not supported by this adapter. Suggestions are complete single-quoted shell literals. The script retains standard Git completion functions and their options; it refuses to replace unsupported custom completion specifications.

Bash 3.2 can run the adapter only when the existing completion specification has no default/filename fallback options. Otherwise activation is refused because that Bash version cannot suppress those options around a written message. Use Zsh on macOS, or install a newer Bash, for standard Git completion coexistence.

Readline's default Tab binding lists multiple matches. To cycle and select them with Tab, optionally add this explicit binding to `~/.bashrc`:

```bash
bind '"\t":menu-complete'
```

That binding applies to all Readline completions. Remove it to restore the normal binding in a new terminal.

### Fish

```fish
mkdir -p "$HOME/.config/git-autocomplete"
git-autocomplete shell fish > "$HOME/.config/git-autocomplete/init.fish"
```

Add to `~/.config/fish/config.fish`:

```fish
source "$HOME/.config/git-autocomplete/init.fish"
```

Fish 3.4+ uses additive option-argument completions, keeps bundled Git completion, and handles escaping in its native pager. Bare arguments and closed empty quotes are supported; incomplete quote contexts may return no suggestions.

### PowerShell

PowerShell 7.3+ is required so empty arguments reach native executables reliably. Run:

```powershell
$config = Join-Path $HOME '.config/git-autocomplete'
New-Item -ItemType Directory -Force $config | Out-Null
git-autocomplete shell powershell | Set-Content -Encoding utf8 (Join-Path $config init.ps1)
```

Add to `$PROFILE`, after importing posh-git if you use it:

```powershell
. "$HOME/.config/git-autocomplete/init.ps1"
```

The native argument completer quotes messages as PowerShell single-quoted literals. It supports the cursor inside closed empty quotes. When posh-git is already imported, other requests delegate to its exported `Expand-GitCommand` function.

PowerShell has one native completer slot per command and no supported API to retrieve another registered completer. For a different Git completer, explicitly pass its existing scriptblock to preserve it:

```powershell
. "$HOME/.config/git-autocomplete/init.ps1" -FallbackCompleter $YourExistingGitCompleter
```

Without that fallback, an unrelated native Git completer is replaced. The script does not inspect private PowerShell internals.

## Verify and use

Open a new terminal in a Git repository, stage a README edit, and run:

```sh
git-autocomplete status
git-autocomplete suggest
git-autocomplete suggest --format json
```

Then type `git commit -m ` with a trailing space and press Tab. Select a suggestion using your shell's normal completion controls. Check the message before pressing Enter. Press Ctrl+C to cancel.

```json
{"suggestions":[{"message":"docs: update README","confidence":0.95,"reason":"Documentation paths identify the change category"}]}
```

Other commands are `help`, `version`, and `shell zsh|bash|fish|powershell`. `status` returns the repository root, staged file count, version, and offline flag as JSON. `suggest` emits one message per line by default, or an empty list in JSON when nothing is staged. Errors go to stderr with a nonzero exit code. Shell requests hide diagnostics and fall back to existing completion on errors.

Autocomplete handles the separate `-m` and `--message` forms, optional repeated literal `git -C directory`, and common flags such as `--amend` and `--signoff`. It intentionally skips aliases, variable expressions, global `-c`, attached message options, pathspecs, multiple message paragraphs, `-F`, and changes selected through `-a`, `--only`, or `--include`. Those contexts can commit a different set of changes than the index snapshot. Git itself continues to work normally.

## Rules and confidence

Confidence is an ordinal rule score divided by 100, **not a probability**. Fixed tiers allow stable ordering. Messages with stronger evidence come first; the MVP returns one or two distinct suggestions, never filler to reach three.

| Score | Evidence |
| --- | --- |
| 0.95 | Documentation, CI, dependency paths, or identical-content rename |
| 0.90 | Test-only paths |
| 0.85 | Binary content or changes consisting only of recognized imports |
| 0.80 | Added public/exported function, or configuration path category |
| 0.75 | Recognized nonpublic function addition, removal, changed declaration, or affected behavior named by function context |
| 0.55 | Source path summary with unknown intent |
| 0.50 | Generic or mixed path summary with unknown intent |

Path rules cover docs, tests, dependency manifests, lockfiles, CI, and configuration. Declaration and import rules cover JavaScript, TypeScript, PHP, Python, Rust, Go, and Java. Unknown extensions get a path summary. Source changes take priority over accompanying tests. A scope is used only when the primary files share a safe, meaningful directory or stem.

For up to eight source files, the engine reads function declarations and enclosing function names in Git's diff hunk headers. Names such as `validateLogin` and `savePushSubscription` become descriptions such as `update login validation` and `update push subscription persistence`. Multiple files can contribute to one behavior summary. These names identify the affected code; they do not establish why the developer changed it. When that evidence is missing or analysis limits are exceeded, the fallback names the affected implementations instead of a generic directory such as `apps`.

Declaration rules are line-based. Multiline declarations may be missed, and examples inside multiline strings or comments can resemble declarations. Review suggestions before committing.

Conventional Commit types are `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `style`, `build`, `ci`, and `chore`. This release emits `refactor` for identical-content source renames and may emit `feat` for an added public function. It does not infer bug fixes, performance gains, or formatting-only intent. `fix`, `perf`, and `style` remain reserved for future rules with defensible evidence. A diff cannot establish the developer's intent, and even a public function can be an internal implementation detail. That is why a public-function suggestion includes a conservative alternative.

## Limits and safety

- Analysis reads only the index compared with HEAD, including unborn repositories and detached HEAD. Untracked and unstaged changes do not influence suggestions. Unresolved conflicts return an error.
- Metadata is capped at 4 MiB. Detailed analysis inspects at most eight primary source files with a combined patch budget of 256 KiB. Changes beyond either limit get a path summary. Each Git process has a two-second timeout. Requests use two Git processes plus at most one patch process per source file, with up to four patch reads running concurrently.
- Rename detection uses 90% similarity and a 200-candidate limit. Identical renames get a precise description; other renames get conservative summaries. The tool does not analyze submodule contents.
- Filenames are parsed as NUL-delimited bytes. Unix non-UTF-8 paths work and display replacement characters. Windows requires UTF-8 paths from Git. Terminal controls and directional formatting characters are removed from descriptions. Long labels are shortened to 48 characters.
- No shell commands are built from paths or diff contents. Git receives argument arrays and literal pathspecs. External diff, textconv, fsmonitor helpers, pagers, and lazy object fetching are disabled. Transport protocols are blocked as a second guard against fetching missing partial-clone objects. Missing objects produce an error.
- Suggestions do not use branch names or commit history to invent intent. Concurrent index edits can race a completion request; rerun completion after staging finishes.

The warm latency goal is under 100 ms for ordinary repositories. It is not a guarantee for slow filesystems or large repositories. Measure on your machine with:

```sh
cargo build --release --locked --offline
python3 scripts/benchmark.py
```

The benchmark creates disposable repositories with one, eight, and 2,000 staged files and reports median and p95 wall time. See [verification notes](VERIFICATION.md) for local measurements and tested platforms.

## Update and uninstall

For a source install, update the checkout and rerun `cargo install --path . --locked --offline --force`. For an archive install, verify the new checksum and replace the executable. Regenerate your shell integration file with the matching `shell` command after either kind of update, then open a new terminal to load it.

To uninstall, remove the source line from your shell startup file, delete `~/.config/git-autocomplete`, then open a new terminal to restore the previous completion. Run `cargo uninstall git-autocomplete` for Cargo installs. For archive installs, remove `~/.local/bin/git-autocomplete` or `%LOCALAPPDATA%\git-autocomplete`, and remove a PATH entry if you added one exclusively for this tool. Remove the optional Bash menu binding if you enabled it.

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md) for tests and detection rules, [demo instructions](scripts/DEMO.md) for a disposable walkthrough, and [CHANGELOG.md](CHANGELOG.md) for release notes. MIT licensed.
