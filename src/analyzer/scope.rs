use super::files;
use crate::git::Change;

fn candidate(path: &str) -> Option<String> {
    let mut parts = path.split('/').filter(|p| {
        !matches!(
            *p,
            "src"
                | "lib"
                | "app"
                | "apps"
                | "packages"
                | "crates"
                | "tests"
                | "test"
                | "__tests__"
                | "spec"
                | "specs"
                | "main"
                | "java"
        )
    });
    let first = parts.next()?;
    let value = if parts.next().is_some() {
        first.to_string()
    } else {
        files::stem(first)
    };
    let value = value.to_ascii_lowercase();
    if value.is_empty()
        || value.len() > 24
        || !value.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        || matches!(
            value.as_str(),
            "main" | "lib" | "index" | "mod" | "init" | "__init__"
        )
    {
        return None;
    }
    Some(value)
}

pub fn infer(changes: &[&Change]) -> Option<String> {
    let first = candidate(&changes.first()?.name())?;
    changes
        .iter()
        .all(|c| candidate(&c.name()).as_ref() == Some(&first))
        .then_some(first)
}

#[cfg(test)]
mod tests {
    #[test]
    fn useful_safe_scopes() {
        assert_eq!(
            super::candidate("src/auth/login.ts").as_deref(),
            Some("auth")
        );
        assert_eq!(super::candidate("src/lib.rs"), None);
        assert_eq!(super::candidate("src/a): bad/login.rs"), None);
        assert_eq!(
            super::candidate("apps/api/src/worker.ts").as_deref(),
            Some("api")
        );
    }
}
