#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Source,
    Test,
    Docs,
    Deps,
    Ci,
    Config,
    Other,
}

pub fn filename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

pub fn stem(path: &str) -> String {
    let file = filename(path);
    let base = file
        .rsplit_once('.')
        .filter(|(stem, _)| !stem.is_empty())
        .map_or(file, |(stem, _)| stem);
    let base = base
        .strip_suffix(".test")
        .or_else(|| base.strip_suffix(".spec"))
        .or_else(|| base.strip_prefix("test_"))
        .or_else(|| base.strip_suffix("_test"))
        .unwrap_or(base);
    super::label(base)
}

pub fn classify(path: &str) -> Kind {
    let lower = path.to_ascii_lowercase();
    let file = filename(&lower);
    let ext = file.rsplit('.').next().unwrap_or("");
    if lower.starts_with(".github/workflows/")
        || lower.starts_with(".circleci/")
        || matches!(
            file,
            ".gitlab-ci.yml" | "jenkinsfile" | ".travis.yml" | "azure-pipelines.yml"
        )
    {
        return Kind::Ci;
    }
    if matches!(
        file,
        "cargo.toml"
            | "cargo.lock"
            | "package.json"
            | "package-lock.json"
            | "yarn.lock"
            | "pnpm-lock.yaml"
            | "bun.lock"
            | "bun.lockb"
            | "composer.json"
            | "composer.lock"
            | "requirements.txt"
            | "pyproject.toml"
            | "poetry.lock"
            | "uv.lock"
            | "pipfile"
            | "pipfile.lock"
            | "go.mod"
            | "go.sum"
            | "pom.xml"
            | "build.gradle"
            | "build.gradle.kts"
            | "gemfile"
            | "gemfile.lock"
    ) {
        return Kind::Deps;
    }
    if lower
        .split('/')
        .any(|p| matches!(p, "test" | "tests" | "__tests__" | "spec" | "specs"))
        || file.contains(".test.")
        || file.contains(".spec.")
        || file.starts_with("test_")
        || file.ends_with("_test.go")
        || file.ends_with("_test.py")
        || filename(path).ends_with("Test.java")
        || filename(path).ends_with("Test.php")
    {
        return Kind::Test;
    }
    if lower.starts_with("docs/")
        || matches!(ext, "md" | "mdx" | "rst" | "adoc")
        || file.starts_with("readme")
        || matches!(file, "license" | "copying")
    {
        return Kind::Docs;
    }
    if file.contains("config")
        || file.starts_with(".env")
        || matches!(
            ext,
            "yml" | "yaml" | "toml" | "ini" | "json" | "conf" | "properties"
        )
        || matches!(
            file,
            "makefile" | "dockerfile" | ".gitignore" | ".gitattributes" | ".editorconfig"
        )
    {
        return Kind::Config;
    }
    if matches!(
        ext,
        "js" | "jsx"
            | "mjs"
            | "cjs"
            | "ts"
            | "tsx"
            | "php"
            | "py"
            | "rs"
            | "go"
            | "java"
            | "c"
            | "h"
            | "cpp"
            | "hpp"
            | "cs"
            | "swift"
            | "kt"
            | "rb"
            | "sh"
            | "css"
            | "scss"
            | "html"
            | "vue"
            | "svelte"
    ) {
        return Kind::Source;
    }
    Kind::Other
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paths_classify_before_extensions() {
        for (path, expected) in [
            ("README.md", Kind::Docs),
            (".github/workflows/test.yml", Kind::Ci),
            ("package-lock.json", Kind::Deps),
            ("src/auth/login.ts", Kind::Source),
            ("tests/user.test.ts", Kind::Test),
            ("src/Contest.java", Kind::Source),
            ("tests/data.json", Kind::Test),
            ("new.xyz", Kind::Other),
        ] {
            assert_eq!(classify(path), expected, "{path}");
        }
    }
}
