use crate::{generator, git, json_string};
use std::ffi::OsString;
use std::path::PathBuf;

const HELP: &str = "git-autocomplete\n\nOffline suggestions from the Git index. Never stages files or executes commits.\n\nUsage:\n  git-autocomplete suggest [--format text|json]\n  git-autocomplete status\n  git-autocomplete version\n  git-autocomplete help\n  git-autocomplete shell zsh|bash|fish|powershell\n\nSource the matching script from completions/ to activate native Git completion.\n";

pub fn run(args: &[OsString]) -> Result<String, String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let strings: Vec<_> = args.iter().map(|a| a.to_str().unwrap_or("")).collect();
    match strings.as_slice() {
        [] | ["help"] | ["--help"] | ["-h"] => Ok(HELP.into()),
        ["version"] | ["--version"] | ["-V"] => {
            Ok(format!("git-autocomplete {}\n", env!("CARGO_PKG_VERSION")))
        }
        ["suggest"] | ["suggest", "--format", "text"] => {
            let snapshot = git::read_staged(&cwd)?;
            Ok(lines(&generator::generate(&snapshot)?))
        }
        ["suggest", "--format", "json"] => {
            let snapshot = git::read_staged(&cwd)?;
            Ok(generator::json(&generator::generate(&snapshot)?))
        }
        ["status"] => {
            let snapshot = git::read_staged(&cwd)?;
            Ok(format!(
                "{{\"repository\":{},\"staged_files\":{},\"offline\":true,\"version\":{}}}\n",
                json_string(&snapshot.root.to_string_lossy()),
                snapshot.changes.len(),
                json_string(env!("CARGO_PKG_VERSION"))
            ))
        }
        ["shell", shell] => {
            let script = match *shell {
                "zsh" => include_str!("../completions/git-autocomplete.zsh"),
                "bash" => include_str!("../completions/git-autocomplete.bash"),
                "fish" => include_str!("../completions/git-autocomplete.fish"),
                "powershell" => include_str!("../completions/git-autocomplete.ps1"),
                _ => return Err("shell must be zsh, bash, fish, or powershell".into()),
            };
            Ok(script.into())
        }
        ["complete", shell, "--", ..]
            if matches!(*shell, "zsh" | "bash" | "fish" | "powershell") =>
        {
            let directory = completion_directory(&args[3..], cwd)?;
            let snapshot = git::read_staged(&directory)?;
            let suggestions = generator::generate(&snapshot)?;
            if *shell == "bash" {
                Ok(suggestions
                    .iter()
                    .map(|s| format!("{}\n", quote_posix(&s.message)))
                    .collect())
            } else {
                Ok(lines(&suggestions))
            }
        }
        _ => Err("invalid arguments; run git-autocomplete help".into()),
    }
}

fn lines(suggestions: &[generator::Suggestion]) -> String {
    suggestions
        .iter()
        .map(|s| format!("{}\n", s.message))
        .collect()
}

/// Completion is deliberately narrower than Git's parser. Unknown contexts delegate to Git.
fn completion_directory(words: &[OsString], mut cwd: PathBuf) -> Result<PathBuf, String> {
    let invalid = || "not an empty Git commit message argument".to_string();
    if words.len() < 4
        || !matches!(words[0].to_str(), Some("git" | "git.exe"))
        || !words.last().is_some_and(|w| w.is_empty())
    {
        return Err(invalid());
    }
    let mut i = 1;
    while words.get(i).is_some_and(|w| w == "-C") {
        let path = words.get(i + 1).ok_or_else(invalid)?;
        if !path.is_empty() {
            cwd.push(path);
        }
        i += 2;
    }
    if !words.get(i).is_some_and(|w| w == "commit") {
        return Err(invalid());
    }
    i += 1;
    while i < words.len() - 2 {
        match words[i].to_str() {
            Some(
                "--amend" | "--signoff" | "-s" | "--no-verify" | "-n" | "--quiet" | "-q"
                | "--verbose" | "-v" | "--allow-empty" | "--no-gpg-sign" | "--no-post-rewrite",
            ) => i += 1,
            _ => return Err(invalid()),
        }
    }
    if i != words.len() - 2 || !matches!(words[i].to_str(), Some("-m" | "--message")) {
        return Err(invalid());
    }
    Ok(cwd)
}

fn quote_posix(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn words(args: &[&str]) -> Vec<OsString> {
        args.iter().map(|arg| OsString::from(*arg)).collect()
    }
    #[test]
    fn only_empty_messages_in_supported_contexts() {
        for args in [
            vec!["git", "commit", "-m", ""],
            vec![
                "git",
                "-C",
                "child dir",
                "commit",
                "--amend",
                "--message",
                "",
            ],
        ] {
            assert!(completion_directory(&words(&args), PathBuf::from("/repo")).is_ok());
        }
        for args in [
            vec!["git", "commit", "-m", "mine"],
            vec!["git", "merge", "-m", ""],
            vec!["git", "commit", "-m", "mine", "-m", ""],
            vec!["git", "commit", "--", "-m", ""],
            vec!["git", "commit", "-a", "-m", ""],
            vec!["git", "-c", "alias.commit=!evil", "commit", "-m", ""],
            vec!["git", "commit", "-F", "message", "-m", ""],
        ] {
            assert!(
                completion_directory(&words(&args), PathBuf::new()).is_err(),
                "{args:?}"
            );
        }
    }
    #[test]
    fn shell_literal_escaping() {
        assert_eq!(
            quote_posix("docs: update Bob's $(touch pwn); `x` \\\""),
            "'docs: update Bob'\\''s $(touch pwn); `x` \\\"'"
        );
    }
}
