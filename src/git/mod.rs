mod staged;

pub use staged::{read_staged, Change, Snapshot};

use std::io::{self, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub const METADATA_LIMIT: usize = 4 * 1024 * 1024;
pub const PATCH_LIMIT: usize = 256 * 1024;
const TIMEOUT: Duration = Duration::from_secs(2);

pub struct Output {
    pub bytes: Vec<u8>,
    pub truncated: bool,
}

/// Only invoke read-only Git operations. Disable executable helpers and lazy object fetching.
pub fn command(cwd: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.current_dir(cwd)
        .args([
            "--no-pager",
            "--no-optional-locks",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.quotePath=true",
            "-c",
            "diff.external=",
            "-c",
            "protocol.allow=never",
        ])
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_ALLOW_PROTOCOL", "")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_LITERAL_PATHSPECS", "0")
        .env("GIT_GLOB_PATHSPECS", "0")
        .env("GIT_NOGLOB_PATHSPECS", "0")
        .env("GIT_ICASE_PATHSPECS", "0")
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd
}

/// Bounded pipes prevent an enormous patch or error message from filling memory.
pub fn run(mut cmd: Command, limit: usize) -> Result<Output, String> {
    let mut child = cmd.spawn().map_err(|e| format!("cannot start Git: {e}"))?;
    let stdout = child.stdout.take().ok_or("missing Git stdout")?;
    let stderr = child.stderr.take().ok_or("missing Git stderr")?;
    let out = thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .take((limit + 1) as u64)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let err = thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stderr.take(4096).read_to_end(&mut bytes);
        bytes
    });
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) if started.elapsed() < TIMEOUT => thread::sleep(Duration::from_millis(2)),
            result => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(match result {
                    Err(e) => format!("cannot wait for Git: {e}"),
                    _ => "Git exceeded the two-second time limit".to_string(),
                });
            }
        }
    };
    let mut bytes = out
        .join()
        .map_err(|_| "Git output reader failed")?
        .map_err(|e: io::Error| format!("cannot read Git output: {e}"))?;
    let stderr = err.join().map_err(|_| "Git error reader failed")?;
    let status = status?;
    let truncated = bytes.len() > limit;
    bytes.truncate(limit);
    if !status.success() && !truncated {
        let detail: String = String::from_utf8_lossy(&stderr)
            .chars()
            .filter(|c| !c.is_control())
            .take(300)
            .collect();
        return Err(format!("Git failed: {detail}"));
    }
    Ok(Output { bytes, truncated })
}
