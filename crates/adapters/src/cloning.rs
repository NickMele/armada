//! `git clone`, run for a person, bounded, and never left half-written.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use adapter_traits::NotCloned;

/// How often a running clone is asked whether it has finished.
const POLL: Duration = Duration::from_millis(50);

/// **The command line rather than git2**, so a clone authenticates the way the
/// person's own shell would — their helper, their agent, their config.
pub(crate) fn clone_repository(
    url: &str,
    destination: &str,
    within: Duration,
) -> Result<(), NotCloned> {
    let existed = Path::new(destination).exists();
    let mut child = Command::new("git")
        // `--` so a URL or folder beginning with `-` is never read as a flag.
        .args(["clone", "--quiet", "--", url, destination])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .env("GIT_TERMINAL_PROMPT", "0")
        .spawn()
        .map_err(|why| NotCloned::NotRun {
            said: why.to_string(),
        })?;
    let mut stderr = child.stderr.take().expect("stderr is piped");
    // Read on its own thread so a chatty git cannot fill the pipe and stall.
    let reading = std::thread::spawn(move || {
        let mut said = String::new();
        let _ = stderr.read_to_string(&mut said);
        said
    });
    let deadline = Instant::now() + within;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(_)) => {
                let said = reading.join().unwrap_or_default();
                return Err(NotCloned::Refused {
                    said: said.trim().to_string(),
                });
            }
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                undo(destination, existed);
                return Err(NotCloned::TookTooLong { waited: within });
            }
            Ok(None) => std::thread::sleep(POLL),
            Err(why) => {
                return Err(NotCloned::NotRun {
                    said: why.to_string(),
                })
            }
        }
    }
}

/// A killed git cleans up nothing. What was there before was empty or absent,
/// so this puts it back as it was.
fn undo(destination: &str, existed: bool) {
    if !existed {
        let _ = std::fs::remove_dir_all(destination);
        return;
    }
    let Ok(entries) = std::fs::read_dir(destination) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let _ = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
    }
}
