use super::{command, run, METADATA_LIMIT, PATCH_LIMIT};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Change {
    pub status: char,
    pub path: Vec<u8>,
    pub old_path: Option<Vec<u8>>,
    pub identical_rename: bool,
}

impl Change {
    pub fn name(&self) -> String {
        String::from_utf8_lossy(&self.path).into_owned()
    }
}

#[derive(Debug)]
pub struct Snapshot {
    pub root: PathBuf,
    pub changes: Vec<Change>,
}

#[cfg(unix)]
fn os_path(bytes: Vec<u8>) -> Result<OsString, String> {
    use std::os::unix::ffi::OsStringExt;
    Ok(OsString::from_vec(bytes))
}

#[cfg(not(unix))]
fn os_path(bytes: Vec<u8>) -> Result<OsString, String> {
    String::from_utf8(bytes)
        .map(OsString::from)
        .map_err(|_| "Git returned a path that is not UTF-8 on this platform".to_string())
}

pub fn read_staged(cwd: &Path) -> Result<Snapshot, String> {
    let mut root_cmd = command(cwd);
    root_cmd.args(["rev-parse", "--show-toplevel"]);
    let mut root = run(root_cmd, 32768)?;
    if root.truncated {
        return Err("repository path exceeds the size limit".into());
    }
    if root.bytes.last() == Some(&b'\n') {
        root.bytes.pop();
    }
    #[cfg(windows)]
    if root.bytes.last() == Some(&b'\r') {
        root.bytes.pop();
    }
    let root = PathBuf::from(os_path(root.bytes)?);
    let mut cmd = command(&root);
    cmd.args([
        "diff",
        "--cached",
        "--name-status",
        "-z",
        "--no-relative",
        "--no-color",
        "--no-ext-diff",
        "--no-textconv",
        "--ignore-submodules=none",
        "--find-renames=90%",
        "-l200",
        "--",
    ]);
    let output = run(cmd, METADATA_LIMIT)?;
    if output.truncated {
        return Err("staged file list exceeds 4 MiB; completion skipped".into());
    }
    Ok(Snapshot {
        root,
        changes: parse(&output.bytes)?,
    })
}

impl Snapshot {
    pub fn patch(&self, change: &Change) -> Result<super::Output, String> {
        let mut pathspec = b":(top,literal)".to_vec();
        pathspec.extend_from_slice(&change.path);
        let mut cmd = command(&self.root);
        cmd.args([
            "diff",
            "--cached",
            "--no-ext-diff",
            "--no-textconv",
            "--no-color",
            "--no-relative",
            "--no-renames",
            "--ignore-submodules=none",
            "--submodule=short",
            "--unified=0",
            "--diff-algorithm=myers",
            "--no-indent-heuristic",
            "--",
        ])
        .arg(os_path(pathspec)?);
        run(cmd, PATCH_LIMIT)
    }
}

fn parse(bytes: &[u8]) -> Result<Vec<Change>, String> {
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    if bytes.last() != Some(&0) {
        return Err("incomplete Git name-status output".into());
    }
    let mut fields = bytes[..bytes.len() - 1].split(|b| *b == 0);
    let mut result = Vec::new();
    while let Some(status) = fields.next() {
        let code = *status.first().ok_or("empty Git status")? as char;
        if !"AMDRCTUXB".contains(code) {
            return Err("unknown Git change status".into());
        }
        let first = fields
            .next()
            .filter(|p| !p.is_empty())
            .ok_or("missing Git path")?
            .to_vec();
        let (old_path, path) = if code == 'R' || code == 'C' {
            (
                Some(first),
                fields
                    .next()
                    .filter(|p| !p.is_empty())
                    .ok_or("missing rename path")?
                    .to_vec(),
            )
        } else {
            (None, first)
        };
        result.push(Change {
            status: code,
            path,
            old_path,
            identical_rename: status == b"R100",
        });
    }
    // An unresolved merge does not have a complete staged snapshot to describe.
    if result.iter().any(|c| c.status == 'U') {
        return Err("resolve staged merge conflicts first".into());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nul_paths_and_rename() {
        let changes = parse(b"A\0a\nb\t.md\0R100\0old name\0new name\0").unwrap();
        assert_eq!(changes[0].path, b"a\nb\t.md");
        assert_eq!(changes[1].old_path.as_deref(), Some(b"old name".as_slice()));
        assert!(changes[1].identical_rename);
        assert!(parse(b"A\0missing terminator").is_err());
        assert!(parse(b"R100\0old\0").is_err());
        assert!(parse(b"U\0conflict\0").is_err());
    }
}
