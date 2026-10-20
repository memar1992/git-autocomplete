use std::collections::{BTreeMap, BTreeSet};

#[derive(Default, Debug)]
pub struct Evidence {
    pub added: BTreeMap<String, bool>,
    pub removed: BTreeMap<String, bool>,
    pub touched: BTreeSet<String>,
    pub imports_only: bool,
    pub binary: bool,
}

fn identifier(text: &str) -> Option<String> {
    let name: String = text
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(*c, '_' | '$'))
        .collect();
    let first = name.chars().next()?;
    if text[name.len()..]
        .chars()
        .next()
        .is_some_and(|c| !c.is_ascii())
    {
        return None;
    }
    (name.len() <= 48 && (first.is_ascii_alphabetic() || first == '_')).then_some(name)
}

/// Recognize declarations, never infer intent from words inside strings or comments.
fn declaration(line: &str, ext: &str) -> Option<(String, bool)> {
    let s = line.trim_start();
    match ext {
        "rs" => {
            let public = s.starts_with("pub ");
            let s = s.strip_prefix("pub ").unwrap_or(s);
            let s = if s.starts_with("pub(") {
                s.split_once(") ")?.1
            } else {
                s
            };
            let s = s.strip_prefix("async ").unwrap_or(s);
            let s = s.strip_prefix("unsafe ").unwrap_or(s);
            let s = s.strip_prefix("const ").unwrap_or(s);
            Some((identifier(s.strip_prefix("fn ")?)?, public))
        }
        "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" => {
            let public = s.starts_with("export ");
            let s = s.strip_prefix("export ").unwrap_or(s);
            let s = s.strip_prefix("default ").unwrap_or(s);
            let s = s.strip_prefix("async ").unwrap_or(s);
            if let Some(rest) = s.strip_prefix("function ") {
                return Some((identifier(rest)?, public));
            }
            if s.contains("=>") {
                let rest = s
                    .strip_prefix("const ")
                    .or_else(|| s.strip_prefix("let "))?;
                let (_, expression) = rest.split_once('=')?;
                let expression = expression.trim_start();
                let expression = expression.strip_prefix("async ").unwrap_or(expression);
                let parameters = expression.split_once("=>")?.0.trim();
                if (parameters.starts_with('(') && parameters.ends_with(')'))
                    || identifier(parameters).as_deref() == Some(parameters)
                {
                    return Some((identifier(rest)?, public));
                }
            }
            None
        }
        "py" => {
            let rest = s.strip_prefix("async ").unwrap_or(s).strip_prefix("def ")?;
            Some((identifier(rest)?, false))
        }
        "php" => {
            let public = s.starts_with("public ");
            let mut rest = s;
            for prefix in [
                "public ",
                "protected ",
                "private ",
                "static ",
                "final ",
                "abstract ",
            ] {
                rest = rest.strip_prefix(prefix).unwrap_or(rest);
            }
            Some((
                identifier(rest.strip_prefix("function ")?.trim_start_matches('&'))?,
                public,
            ))
        }
        "go" => {
            let rest = s.strip_prefix("func ")?;
            let rest = if rest.starts_with('(') {
                rest.split_once(')')?.1.trim_start()
            } else {
                rest
            };
            let name = identifier(rest)?;
            let public = name.chars().next()?.is_ascii_uppercase();
            Some((name, public))
        }
        "java" => {
            if !s.starts_with("public ")
                && !s.starts_with("private ")
                && !s.starts_with("protected ")
            {
                return None;
            }
            let head = s.split_once('(')?.0;
            if head.contains('=') || head.contains(" class ") || head.contains(" interface ") {
                return None;
            }
            let name = identifier(head.split_whitespace().last()?)?;
            Some((name, s.starts_with("public ")))
        }
        _ => None,
    }
}

fn import(line: &str, ext: &str) -> bool {
    let s = line.trim();
    match ext {
        "rs" => s.starts_with("use ") || s.starts_with("pub use "),
        "py" => s.starts_with("import ") || (s.starts_with("from ") && s.contains(" import ")),
        "js" | "ts" | "jsx" | "tsx" | "mjs" | "cjs" => {
            s.starts_with("import ")
                || ((s.starts_with("const ") || s.starts_with("let ")) && s.contains("require("))
        }
        "java" => s.starts_with("import "),
        "php" => {
            s.starts_with("use ") || s.starts_with("require ") || s.starts_with("require_once ")
        }
        "go" => s.starts_with("import ") && !s.ends_with('('),
        _ => false,
    }
}

pub fn analyze(patch: &[u8], path: &str) -> Evidence {
    let ext = path.rsplit('.').next().unwrap_or("");
    let mut evidence = Evidence::default();
    let mut in_hunk = false;
    let mut changed = 0;
    let mut imports = 0;
    for line in String::from_utf8_lossy(patch).lines() {
        if line.starts_with("diff --git ") {
            in_hunk = false;
        }
        if line.starts_with("Binary files ") || line == "GIT binary patch" {
            evidence.binary = true;
        }
        if line.starts_with("@@ ") {
            in_hunk = true;
            if let Some((_, context)) = line.split_once(" @@") {
                if let Some((name, _)) = declaration(context.trim_start(), ext) {
                    evidence.touched.insert(name);
                }
            }
            continue;
        }
        if !in_hunk {
            continue;
        }
        let (added, text) = match line.as_bytes().first() {
            Some(b'+') => (true, &line[1..]),
            Some(b'-') => (false, &line[1..]),
            _ => continue,
        };
        if text.trim().is_empty() {
            continue;
        }
        changed += 1;
        if import(text, ext) {
            imports += 1;
        }
        if let Some((name, public)) = declaration(text, ext) {
            if added {
                evidence.added.insert(name, public);
            } else {
                evidence.removed.insert(name, public);
            }
        }
    }
    evidence.imports_only = changed > 0 && changed == imports;
    evidence
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seven_languages() {
        for (ext, line, public) in [
            ("js", "export function validate() {", true),
            ("ts", "export const validate = () => 1", true),
            ("php", "public function validate() {", true),
            ("py", "def validate(user):", false),
            ("rs", "pub fn validate() {", true),
            ("go", "func Validate() {", true),
            ("java", "public boolean validate(User user) {", true),
        ] {
            let (name, actual) = declaration(line, ext).unwrap();
            assert!(name.eq_ignore_ascii_case("validate"));
            assert_eq!(actual, public);
        }
        assert!(declaration("// export function fake()", "js").is_none());
        assert!(declaration("export const example = '=>';", "ts").is_none());
        assert!(declaration("export const example = { get: () => 1 };", "ts").is_none());
        assert!(declaration("pub fn café() {}", "rs").is_none());
        assert!(declaration("pub(crate) fn internal() {", "rs").is_some_and(|(_, public)| !public));
    }
    #[test]
    fn only_hunks_and_imports_count() {
        let e = analyze(
            b"+++ b/export function fake()\n@@ -1 +1 @@\n-import old\n+import new\n",
            "a.py",
        );
        assert!(e.imports_only);
        assert!(e.added.is_empty());
        let e = analyze(b"@@ -0,0 +1 @@\n+def validate(x):\n", "a.py");
        assert!(e.added.contains_key("validate"));
    }

    #[test]
    fn function_body_context_is_not_a_new_declaration() {
        let e = analyze(
            b"@@ -4 +4 @@ export async function savePushSubscription(subscription) {\n-  return false;\n+  return true;\n",
            "src/push.ts",
        );
        assert!(e.touched.contains("savePushSubscription"));
        assert!(e.added.is_empty() && e.removed.is_empty());
        let e = analyze(b"@@ -1 +1 @@\n-const x = 1;\n+const x = 2;\n", "a.ts");
        assert!(e.touched.is_empty());
    }
}
