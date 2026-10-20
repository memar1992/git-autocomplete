use git_autocomplete::{generator, git};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Repo {
    path: PathBuf,
}

impl Repo {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "git-autocomplete test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        let repo = Self { path };
        repo.git(&["init", "-q", "--initial-branch=main"]);
        repo.git(&["config", "user.email", "test@example.invalid"]);
        repo.git(&["config", "user.name", "Test"]);
        repo.git(&["config", "core.autocrlf", "false"]);
        repo.git(&["config", "core.hooksPath", ".git/unused-hooks"]);
        repo.git(&["config", "commit.gpgsign", "false"]);
        repo
    }
    fn git(&self, args: &[&str]) -> Output {
        let out = Command::new("git")
            .current_dir(&self.path)
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        out
    }
    fn write(&self, path: &str, bytes: impl AsRef<[u8]>) {
        let path = self.path.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    fn stage(&self) {
        self.git(&["add", "--all"]);
    }
    fn commit(&self) {
        self.stage();
        self.git(&["commit", "-qm", "fixture"]);
    }
    fn suggestions(&self) -> Vec<generator::Suggestion> {
        generator::generate(&git::read_staged(&self.path).unwrap()).unwrap()
    }
    fn cli(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_git-autocomplete"))
            .current_dir(&self.path)
            .args(args)
            .output()
            .unwrap()
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn empty_index_and_new_repository() {
    let repo = Repo::new();
    assert!(repo.suggestions().is_empty());
    assert_eq!(
        repo.cli(&["suggest", "--format", "json"]).stdout,
        b"{\"suggestions\":[]}\n"
    );
    repo.write("README.md", include_str!("fixtures/README.md"));
    repo.stage();
    assert_eq!(repo.suggestions()[0].message, "docs: add README");
}

#[test]
fn changes_are_staged_only_and_never_modified() {
    let repo = Repo::new();
    repo.write("README.md", "old\n");
    repo.commit();
    repo.write("README.md", "staged\n");
    repo.stage();
    repo.write("README.md", "unstaged\n");
    repo.write("src/untracked.rs", "pub fn should_not_appear() {}\n");
    let index = fs::read(repo.path.join(".git/index")).unwrap();
    let files = worktree(&repo.path);
    let first = repo.suggestions();
    assert_eq!(first[0].message, "docs: update README");
    assert_eq!(first, repo.suggestions());
    assert_eq!(index, fs::read(repo.path.join(".git/index")).unwrap());
    assert_eq!(files, worktree(&repo.path));
}

fn worktree(path: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files = Vec::new();
    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name() == ".git" {
            continue;
        }
        if entry.file_type().unwrap().is_dir() {
            files.extend(worktree(&entry.path()));
        } else {
            files.push((entry.path(), fs::read(entry.path()).unwrap()));
        }
    }
    files.sort();
    files
}

#[test]
fn rename_delete_and_detached_head() {
    let repo = Repo::new();
    repo.write("src/auth/old.rs", "pub fn login() {}\n");
    repo.commit();
    repo.git(&["checkout", "--detach", "-q"]);
    repo.git(&["mv", "src/auth/old.rs", "src/auth/new.rs"]);
    assert_eq!(
        repo.suggestions()[0].message,
        "refactor(auth): rename old to new"
    );
    repo.git(&["commit", "-qm", "rename"]);
    repo.git(&["rm", "src/auth/new.rs"]);
    assert_eq!(
        repo.suggestions()[0].message,
        "chore(auth): remove new implementation"
    );
}

#[test]
fn path_categories_and_multiple_files() {
    for (paths, prefix, description) in [
        (vec!["tests/user.test.ts"], "test(user):", "user tests"),
        (vec!["README.md", "docs/setup.md"], "docs:", "documentation"),
        (
            vec!["package.json", "package-lock.json"],
            "build(deps):",
            "dependencies",
        ),
        (vec![".github/workflows/test.yml"], "ci:", "test workflow"),
        (
            vec!["src/auth/login.ts", "src/auth/session.ts"],
            "chore(auth):",
            "login and session implementations",
        ),
        (vec!["settings.yaml"], "chore:", "configuration"),
        (
            vec!["texture.unknown"],
            "chore(texture):",
            "texture.unknown",
        ),
    ] {
        let repo = Repo::new();
        for path in paths {
            repo.write(path, "some content\n");
        }
        repo.stage();
        let result = repo.suggestions();
        assert!(result[0].message.starts_with(prefix), "{result:?}");
        assert!(result[0].message.contains(description), "{result:?}");
    }
}

#[test]
fn implementation_precedes_tests_and_order_is_deterministic() {
    let repo = Repo::new();
    repo.write("src/auth/login.ts", include_str!("fixtures/login.ts"));
    repo.write(
        "tests/auth/login.test.ts",
        include_str!("fixtures/login.test.ts"),
    );
    repo.stage();
    let result = repo.suggestions();
    assert_eq!(result[0].message, "feat(auth): add validateLogin function");
    assert_eq!(result.len(), 2);
    assert!(result[0].score > result[1].score);
    assert_eq!(result, repo.suggestions());
    for pair in result.windows(2) {
        assert_ne!(pair[0].message, pair[1].message);
    }
}

#[test]
fn ambiguous_code_never_claims_fix_or_performance() {
    let repo = Repo::new();
    repo.write("src/auth/login.ts", "let crash = 'fix performance bug';\n");
    repo.stage();
    let result = repo.suggestions();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].message, "chore(auth): add login implementation");
    repo.write("src/auth/login.ts", "export function fixCrash() {}\n");
    repo.write(
        "src/auth/session.ts",
        "export function improvePerformance() {}\n",
    );
    repo.stage();
    let result = repo.suggestions();
    assert_eq!(
        result[0].message,
        "chore(auth): add fixCrash function and improvePerformance function"
    );
}

#[test]
fn function_body_changes_name_the_affected_behavior() {
    let repo = Repo::new();
    repo.write(
        "src/auth/login.ts",
        "export function validateLogin(user) {\n  return user.name;\n}\n",
    );
    repo.commit();
    repo.write(
        "src/auth/login.ts",
        "export function validateLogin(user) {\n  return user.name.trim();\n}\n",
    );
    repo.stage();
    let result = repo.suggestions();
    assert_eq!(result[0].message, "chore(auth): update login validation");
    assert_eq!(result[0].score, 75);
}

#[test]
fn multiple_source_files_describe_behavior_instead_of_app_files() {
    let repo = Repo::new();
    for (path, source) in [
        (
            "apps/api/src/ai.ts",
            "export function generateAIReply() {\n  return result;\n}\n",
        ),
        (
            "apps/api/src/worker.ts",
            "async function processPushNotificationJob() {\n  throw error;\n}\n",
        ),
        (
            "apps/web/src/pwa.tsx",
            "export function PwaProvider() {\n  return enabled;\n}\n",
        ),
        (
            "apps/web/src/components/profile.tsx",
            "export function ProfileSettingsDialog() {\n  return notice;\n}\n",
        ),
    ] {
        repo.write(path, source);
    }
    repo.commit();
    for (path, source) in [
        ("apps/api/src/ai.ts", "export function generateAIReply() {\n  return { arguments: args, result };\n}\n"),
        ("apps/api/src/worker.ts", "async function processPushNotificationJob() {\n  deliveryErrors.push(error);\n}\n"),
        ("apps/web/src/pwa.tsx", "export function PwaProvider() {\n  return subscription ? enabled : disabled;\n}\n"),
        ("apps/web/src/components/profile.tsx", "export function ProfileSettingsDialog() {\n  return enabled ? notice : null;\n}\n"),
        ("apps/web/src/lib/push.ts", "export function savePushSubscription(subscription) {\n  return api(subscription);\n}\n"),
        ("apps/web/tests/pwa.test.tsx", "it('saves subscriptions', () => {});\n"),
    ] {
        repo.write(path, source);
    }
    repo.stage();
    let index = fs::read(repo.path.join(".git/index")).unwrap();
    let files = worktree(&repo.path);
    let result = repo.suggestions();
    assert_eq!(result[0].message,
        "chore: update ai reply generation, push notification job processing and push subscription persistence");
    assert_eq!(result[0].score, 75);
    assert_eq!(result, repo.suggestions());
    assert_eq!(index, fs::read(repo.path.join(".git/index")).unwrap());
    assert_eq!(files, worktree(&repo.path));
}

#[test]
fn multi_file_analysis_respects_file_and_total_patch_limits() {
    let repo = Repo::new();
    for i in 0..8 {
        repo.write(
            &format!("src/part{i}.rs"),
            format!("pub fn validate_part_{i}() {{}}\n"),
        );
    }
    repo.stage();
    let index = fs::read(repo.path.join(".git/index")).unwrap();
    let result = repo.suggestions();
    assert_eq!(result[0].score, 75);
    assert!(result[0].message.ends_with("6 others"));
    assert_eq!(result, repo.suggestions());
    assert_eq!(index, fs::read(repo.path.join(".git/index")).unwrap());
    repo.write("src/part8.rs", "pub fn should_not_be_inferred() {}\n");
    repo.stage();
    assert_eq!(repo.suggestions()[0].score, 55);

    let repo = Repo::new();
    for name in ["one", "two"] {
        repo.write(
            &format!("src/{name}.rs"),
            format!("pub fn misleading() {{}}\n{}", "let v = 1;\n".repeat(15000)),
        );
    }
    repo.stage();
    let result = repo.suggestions();
    assert_eq!(result[0].score, 55);
    assert!(!result[0].message.contains("misleading"));
}

#[test]
fn manifest_metadata_does_not_claim_changed_dependencies() {
    let repo = Repo::new();
    repo.write(
        "Cargo.toml",
        "[package]\nname = \"example\"\nversion = \"0.1.0\"\n",
    );
    repo.stage();
    assert_eq!(
        repo.suggestions()[0].message,
        "build(deps): add dependency manifests"
    );
}

#[test]
fn import_changes_and_function_changes() {
    let repo = Repo::new();
    repo.write(
        "src/auth/login.py",
        "from users import User\ndef login(user):\n    return user\n",
    );
    repo.commit();
    repo.write(
        "src/auth/login.py",
        "from accounts import User\ndef login(user):\n    return user\n",
    );
    repo.stage();
    assert_eq!(repo.suggestions()[0].message, "chore(auth): update imports");
    repo.write(
        "src/auth/login.py",
        "from users import User\ndef login(user, strict=True):\n    return user\n",
    );
    repo.stage();
    assert_eq!(
        repo.suggestions()[0].message,
        "chore(auth): update login function"
    );
    repo.write("src/auth/login.py", "from users import User\n");
    repo.stage();
    assert_eq!(
        repo.suggestions()[0].message,
        "chore(auth): remove login function"
    );
}

#[test]
fn binary_and_bounded_large_patch() {
    let repo = Repo::new();
    repo.write("src/image.rs", b"binary\0data");
    repo.stage();
    assert!(repo.suggestions()[0].message.contains("binary"));
    repo.write(
        "src/image.rs",
        format!("pub fn misleading() {{}}\n{}", "let v = 1;\n".repeat(60000)),
    );
    repo.stage();
    let result = repo.suggestions();
    assert_eq!(result[0].score, 55);
    assert!(!result[0].message.contains("misleading"));
}

#[test]
fn spaces_unicode_controls_and_pathspec_magic() {
    let repo = Repo::new();
    for path in [
        "docs/hello world.md",
        "docs/雪.md",
        "docs/$(echo bad)'quote.md",
        "docs/[glob].md",
    ] {
        repo.write(path, "hello\n");
    }
    repo.stage();
    let snapshot = git::read_staged(&repo.path).unwrap();
    assert_eq!(snapshot.changes.len(), 4);
    assert_eq!(repo.suggestions()[0].message, "docs: add documentation");
    #[cfg(unix)]
    {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        // Some filesystems reject non-UTF-8 names, but Git's index can contain them.
        repo.write("blob-source.rs", "fn unusual() {}\n");
        let blob = repo.git(&["hash-object", "-w", "blob-source.rs"]);
        let oid = std::str::from_utf8(&blob.stdout).unwrap().trim();
        let raw_path = b"src/non-utf8-\xff\n.rs";
        let out = Command::new("git")
            .current_dir(&repo.path)
            .args(["update-index", "--add", "--cacheinfo", "100644", oid])
            .arg(OsStr::from_bytes(raw_path))
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(git::read_staged(&repo.path)
            .unwrap()
            .changes
            .iter()
            .any(|change| change.path == raw_path));
        let result = repo.suggestions();
        assert!(result
            .iter()
            .all(|s| !s.message.contains('\n') && !s.message.contains('\u{1b}')));
        assert!(result[0].message.contains("unusual"));
    }
}

#[test]
fn from_subdirectory_and_git_c_context() {
    let repo = Repo::new();
    repo.write("README.md", "docs\n");
    repo.write("child/file", "data\n");
    repo.stage();
    assert_eq!(
        git::read_staged(&repo.path.join("child"))
            .unwrap()
            .changes
            .len(),
        2
    );
    let out = repo.cli(&[
        "complete",
        "zsh",
        "--",
        "git",
        "-C",
        "child",
        "commit",
        "--message",
        "",
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!out.stdout.is_empty());
    let out = repo.cli(&[
        "complete",
        "bash",
        "--",
        "git",
        "commit",
        "-m",
        "my message",
    ]);
    assert!(!out.status.success());
    assert!(out.stdout.is_empty() && out.stderr.is_empty());
}

#[test]
fn repository_config_cannot_execute_helpers() {
    let repo = Repo::new();
    repo.write("src/login.rs", "pub fn login() {}\n");
    repo.write(".gitattributes", "*.rs diff=unsafe\n");
    repo.stage();
    repo.git(&["config", "diff.external", "this-command-must-never-run"]);
    repo.git(&[
        "config",
        "diff.unsafe.textconv",
        "this-command-must-never-run",
    ]);
    repo.git(&["config", "core.fsmonitor", "this-command-must-never-run"]);
    repo.git(&["config", "core.pager", "this-command-must-never-run"]);
    let result = repo.suggestions();
    assert!(result[0].message.contains("login"));
}

#[test]
fn cli_errors_and_status() {
    let repo = Repo::new();
    let status = repo.cli(&["status"]);
    assert!(status.status.success());
    assert!(String::from_utf8_lossy(&status.stdout).contains("\"staged_files\":0"));
    assert!(!repo.cli(&["suggest", "--format", "bad"]).status.success());
    assert!(repo.cli(&["version"]).status.success());
    assert!(repo.cli(&["help"]).status.success());
}

#[test]
fn staged_gitlink_is_not_hidden() {
    let repo = Repo::new();
    repo.write("README.md", "baseline\n");
    repo.commit();
    let head = String::from_utf8(repo.git(&["rev-parse", "HEAD"]).stdout).unwrap();
    let entry = format!("160000,{},vendor/module", head.trim());
    repo.git(&["update-index", "--add", "--cacheinfo", &entry]);
    let snapshot = git::read_staged(&repo.path).unwrap();
    assert_eq!(snapshot.changes.len(), 1);
    assert_eq!(snapshot.changes[0].path, b"vendor/module");
    assert!(!repo.suggestions().is_empty());
}
