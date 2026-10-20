use crate::analyzer::{
    self,
    files::{self, Kind},
    patterns, scope,
};
use crate::git::{Change, Snapshot};
use std::collections::BTreeSet;

const SOURCE_FILE_LIMIT: usize = 8;

fn words(name: &str) -> Vec<String> {
    let chars: Vec<_> = name.chars().collect();
    let mut text = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c == '_' || c == '-' || c == '$' {
            text.push(' ');
        } else {
            if c.is_ascii_uppercase()
                && i > 0
                && (chars[i - 1].is_ascii_lowercase()
                    || (chars[i - 1].is_ascii_uppercase()
                        && chars.get(i + 1).is_some_and(char::is_ascii_lowercase)))
            {
                text.push(' ');
            }
            text.push(c.to_ascii_lowercase());
        }
    }
    text.split_whitespace().map(str::to_owned).collect()
}

fn behavior(name: &str) -> Option<String> {
    let words = words(name);
    let (verb, subject) = words.split_first()?;
    if subject.is_empty() {
        return None;
    }
    let noun = match verb.as_str() {
        "validate" => "validation",
        "save" => "persistence",
        "process" => "processing",
        "generate" => "generation",
        "send" => "delivery",
        "handle" => "handling",
        "parse" => "parsing",
        "create" => "creation",
        "remove" | "delete" => "removal",
        "enable" => "activation",
        "disable" => "deactivation",
        "load" => "loading",
        "fetch" | "get" => "retrieval",
        "check" => "checks",
        _ => return None,
    };
    Some(format!("{} {noun}", subject.join(" ")))
}

fn summarize(names: Vec<String>) -> String {
    match names.as_slice() {
        [] => String::new(),
        [name] => name.clone(),
        [first, second] => format!("{first} and {second}"),
        [first, second, third] => format!("{first}, {second} and {third}"),
        [first, second, rest @ ..] => format!("{first}, {second} and {} others", rest.len()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub message: String,
    /// An ordinal rule strength from 0 to 100, not a probability.
    pub score: u8,
    pub reason: &'static str,
}

fn suggestion(
    kind: &str,
    scope: Option<&str>,
    description: &str,
    score: u8,
    reason: &'static str,
) -> Suggestion {
    let prefix = match scope {
        Some(scope) => format!("{kind}({scope})"),
        None => kind.into(),
    };
    Suggestion {
        message: format!("{prefix}: {description}"),
        score,
        reason,
    }
}

fn action(changes: &[&Change]) -> &'static str {
    if changes.iter().all(|c| c.status == 'A') {
        "add"
    } else if changes.iter().all(|c| c.status == 'D') {
        "remove"
    } else {
        "update"
    }
}

pub fn generate(snapshot: &Snapshot) -> Result<Vec<Suggestion>, String> {
    if snapshot.changes.is_empty() {
        return Ok(Vec::new());
    }
    let kind = analyzer::primary(&snapshot.changes);
    let focus: Vec<_> = snapshot
        .changes
        .iter()
        .filter(|c| files::classify(&c.name()) == kind)
        .collect();
    let all: Vec<_> = snapshot.changes.iter().collect();
    let focus = if focus.is_empty() { &all } else { &focus };
    let inferred = scope::infer(focus);
    let scope = inferred.as_deref();
    let verb = action(focus);
    let single = focus.len() == 1;
    let name = files::stem(&focus[0].name());
    let name = if name.is_empty() { "project" } else { &name };
    let dependency_label = if focus.iter().any(|c| {
        matches!(
            files::filename(&c.name().to_ascii_lowercase()),
            "cargo.lock"
                | "package-lock.json"
                | "yarn.lock"
                | "pnpm-lock.yaml"
                | "bun.lock"
                | "bun.lockb"
                | "composer.lock"
                | "poetry.lock"
                | "uv.lock"
                | "pipfile.lock"
                | "go.sum"
                | "gemfile.lock"
        )
    }) {
        "dependencies"
    } else {
        "dependency manifests"
    };
    let (commit_type, commit_scope, description, score, reason) = match kind {
        Kind::Docs => (
            "docs",
            None,
            if single {
                format!("{verb} {name}")
            } else {
                format!("{verb} documentation")
            },
            95,
            "Documentation paths identify the change category",
        ),
        Kind::Deps => (
            "build",
            Some("deps"),
            format!("{verb} {dependency_label}"),
            95,
            "Dependency manifest or lockfile changes detected",
        ),
        Kind::Ci => (
            "ci",
            None,
            if single {
                format!("{verb} {name} workflow")
            } else {
                format!("{verb} CI configuration")
            },
            95,
            "CI configuration paths identify the change category",
        ),
        Kind::Test => (
            "test",
            scope,
            if single {
                format!("{verb} {name} tests")
            } else {
                format!("{verb} tests")
            },
            90,
            "Only test paths are staged",
        ),
        Kind::Config => (
            "chore",
            None,
            if single {
                format!("{verb} {name} configuration")
            } else {
                format!("{verb} configuration")
            },
            80,
            "Configuration paths detected; intent is unknown",
        ),
        Kind::Source => (
            "chore",
            scope,
            if single {
                format!("{verb} {name} implementation")
            } else {
                let names: BTreeSet<_> = focus
                    .iter()
                    .map(|c| words(&files::stem(&c.name())).join(" "))
                    .filter(|name| !name.is_empty())
                    .collect();
                format!(
                    "{verb} {} implementations",
                    summarize(names.into_iter().collect())
                )
            },
            55,
            "Source changes detected; developer intent is unknown",
        ),
        Kind::Other => (
            "chore",
            scope,
            if single {
                format!(
                    "{verb} {}",
                    analyzer::label(files::filename(&focus[0].name()))
                )
            } else {
                format!("{verb} project files")
            },
            50,
            "Generic staged path summary; developer intent is unknown",
        ),
    };
    let fallback = suggestion(commit_type, commit_scope, &description, score, reason);
    if single && focus.len() == snapshot.changes.len() && focus[0].identical_rename {
        let old = focus[0].old_path.as_deref().unwrap_or_default();
        let old = files::stem(&String::from_utf8_lossy(old));
        let description = if old == name {
            format!("move {name}")
        } else {
            format!("rename {old} to {name}")
        };
        let rename = suggestion(
            if kind == Kind::Source {
                "refactor"
            } else {
                commit_type
            },
            commit_scope,
            &description,
            95,
            "Git detected a rename with identical content",
        );
        return Ok(vec![rename]);
    }
    if kind != Kind::Source
        || focus.len() > SOURCE_FILE_LIMIT
        || focus.iter().any(|c| matches!(c.status, 'D' | 'R' | 'T'))
    {
        return Ok(vec![fallback]);
    }
    let mut analyses = Vec::new();
    let mut patch_bytes = 0;
    for batch in focus.chunks(4) {
        let patches = std::thread::scope(|scope| {
            let workers: Vec<_> = batch
                .iter()
                .map(|change| scope.spawn(move || snapshot.patch(change)))
                .collect();
            workers
                .into_iter()
                .map(|worker| {
                    worker
                        .join()
                        .map_err(|_| "Git patch worker failed".to_string())?
                })
                .collect::<Result<Vec<_>, String>>()
        })?;
        for (change, patch) in batch.iter().zip(patches) {
            patch_bytes += patch.bytes.len();
            if patch.truncated || patch_bytes > crate::git::PATCH_LIMIT {
                return Ok(vec![fallback]);
            }
            analyses.push(patterns::analyze(&patch.bytes, &change.name()));
        }
    }
    if !single {
        if analyses.iter().all(|e| e.imports_only) {
            return Ok(vec![suggestion(
                "chore",
                scope,
                "update imports",
                85,
                "All changed source lines are import declarations",
            )]);
        }
        let names: BTreeSet<_> = analyses
            .iter()
            .flat_map(|e| {
                e.added
                    .keys()
                    .chain(e.removed.keys())
                    .chain(e.touched.iter())
                    .cloned()
            })
            .collect();
        if !names.is_empty() {
            let mut subjects: Vec<_> = names.iter().filter_map(|name| behavior(name)).collect();
            if subjects.is_empty() {
                subjects = names
                    .iter()
                    .map(|name| format!("{name} function"))
                    .collect();
            }
            subjects.sort();
            subjects.dedup();
            return Ok(vec![suggestion(
                "chore", scope, &format!("{verb} {}", summarize(subjects)), 75,
                "Function declarations and diff context name affected behavior; developer intent is unknown",
            )]);
        }
        return Ok(vec![fallback]);
    }
    let evidence = &analyses[0];
    if evidence.binary {
        return Ok(vec![suggestion(
            "chore",
            scope,
            &format!("{verb} {name} binary"),
            85,
            "Git reports binary content",
        )]);
    }
    if evidence.imports_only {
        return Ok(vec![suggestion(
            "chore",
            scope,
            "update imports",
            85,
            "All changed nonempty lines are import declarations",
        )]);
    }
    let new: Vec<_> = evidence
        .added
        .iter()
        .filter(|(name, _)| !evidence.removed.contains_key(*name))
        .collect();
    if new.len() == 1 && evidence.removed.is_empty() {
        let (function, public) = new[0];
        let specific = suggestion(
            if *public { "feat" } else { "chore" },
            scope,
            &format!("add {function} function"),
            if *public { 80 } else { 75 },
            "An added function declaration was recognized; API visibility determines the type",
        );
        return Ok(if *public {
            vec![specific, fallback]
        } else {
            vec![specific]
        });
    }
    if evidence.added.len() == 1 && evidence.added.keys().eq(evidence.removed.keys()) {
        if let Some(function) = evidence.added.keys().next() {
            return Ok(vec![suggestion(
                "chore",
                scope,
                &format!("update {function} function"),
                75,
                "The same function declaration appears on both sides of the diff",
            )]);
        }
    }
    if evidence.removed.len() == 1 && evidence.added.is_empty() {
        if let Some(function) = evidence.removed.keys().next() {
            return Ok(vec![suggestion(
                "chore",
                scope,
                &format!("remove {function} function"),
                75,
                "A removed function declaration was recognized",
            )]);
        }
    }
    if !evidence.touched.is_empty() {
        let subjects = evidence
            .touched
            .iter()
            .map(|name| behavior(name).unwrap_or_else(|| format!("{name} function")))
            .collect();
        return Ok(vec![suggestion(
            "chore",
            scope,
            &format!("update {}", summarize(subjects)),
            75,
            "Diff hunk context identifies the affected function; developer intent is unknown",
        )]);
    }
    Ok(vec![fallback])
}

pub fn json(suggestions: &[Suggestion]) -> String {
    let entries: Vec<_> = suggestions
        .iter()
        .map(|s| {
            format!(
                "{{\"message\":{},\"confidence\":{:.2},\"reason\":{}}}",
                crate::json_string(&s.message),
                f64::from(s.score) / 100.0,
                crate::json_string(s.reason)
            )
        })
        .collect();
    format!("{{\"suggestions\":[{}]}}\n", entries.join(","))
}

#[cfg(test)]
mod tests {
    #[test]
    fn conventional_format_and_json() {
        let s = super::suggestion("chore", Some("auth"), "update files", 55, "Unknown intent");
        assert_eq!(s.message, "chore(auth): update files");
        assert!(super::json(&[s]).contains("\"confidence\":0.55"));
        assert_eq!(super::json(&[]), "{\"suggestions\":[]}\n");
    }

    #[test]
    fn behavior_labels_describe_symbols_without_claiming_intent() {
        assert_eq!(
            super::behavior("savePushSubscription").as_deref(),
            Some("push subscription persistence")
        );
        assert_eq!(
            super::behavior("generateAIReply").as_deref(),
            Some("ai reply generation")
        );
        assert_eq!(
            super::behavior("validate_login").as_deref(),
            Some("login validation")
        );
        assert_eq!(super::behavior("fixCrash"), None);
        assert_eq!(super::behavior("save"), None);
    }
}
