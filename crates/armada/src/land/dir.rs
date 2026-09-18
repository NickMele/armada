//! Where the merge line keeps its state, and the name a branch is given
//! there.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The six subdirectories every state directory carries, in
/// `scripts/land`'s own `state_dir` order.
const SUBDIRS: [&str; 6] = [
    "queue",
    "outcomes",
    "stamps",
    "logs",
    "foundations",
    "checks",
];

/// `armada-land/`, under a repository's git common directory, with its six
/// subdirectories guaranteed to exist.
///
/// **The common directory, not the worktree's own `.git`.** A worktree has
/// its own `.git` file pointing back at the one this resolves to, so every
/// worktree of one clone shares a single state directory — which is the
/// point: the queue and the turn are for the repository, not the checkout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateDir {
    at: PathBuf,
}

impl StateDir {
    /// Resolve `repo`'s state directory and ensure its subdirectories exist.
    pub fn resolve(repo: &Path) -> Result<StateDir, StateDirError> {
        let common = git_common_dir(repo)?;
        StateDir::ensure(common.join("armada-land"))
    }

    fn ensure(at: PathBuf) -> Result<StateDir, StateDirError> {
        for sub in SUBDIRS {
            let path = at.join(sub);
            std::fs::create_dir_all(&path)
                .map_err(|cause| StateDirError::SubdirectoryUnwritable { path, cause })?;
        }
        Ok(StateDir { at })
    }

    /// A state directory under a common git directory that is already
    /// known — the runner's own case: it is handed the common git
    /// directory directly (`runner::ensure_runner`'s own argv), so
    /// resolving it again with a `git rev-parse` would be asking the
    /// question it was just answered.
    pub fn at_common(common_git_dir: &Path) -> Result<StateDir, StateDirError> {
        StateDir::ensure(common_git_dir.join("armada-land"))
    }

    /// A state directory at an already-chosen path, skipping the git call.
    ///
    /// Test-only: a unit test that only exercises the read/write/merge layer
    /// below has no repository to resolve one from, and paying for a `git
    /// rev-parse` in every one of them would test the shell-out rather than
    /// the state it produces. [`resolve`](StateDir::resolve) is still what
    /// every non-test caller goes through.
    #[cfg(test)]
    pub(crate) fn for_testing(at: PathBuf) -> StateDir {
        StateDir::ensure(at).expect("a fresh temporary directory accepts its own subdirectories")
    }

    pub fn path(&self) -> &Path {
        &self.at
    }

    pub(crate) fn queue_dir(&self) -> PathBuf {
        self.at.join("queue")
    }

    fn outcomes_dir(&self) -> PathBuf {
        self.at.join("outcomes")
    }

    fn stamps_dir(&self) -> PathBuf {
        self.at.join("stamps")
    }

    pub fn queue_entry_path(&self, branch: &str) -> PathBuf {
        self.queue_dir().join(format!("{}.json", key(branch)))
    }

    pub fn outcome_path(&self, branch: &str) -> PathBuf {
        self.outcomes_dir().join(format!("{}.json", key(branch)))
    }

    pub fn stamp_path(&self, branch: &str) -> PathBuf {
        self.stamps_dir().join(format!("{}.json", key(branch)))
    }

    /// Where `base`'s own cached `verify-foundations` report lives —
    /// `scripts/land`'s `base_foundations`'s own cache file, keyed by
    /// commit rather than by branch: every branch gated against the same
    /// `base` reads the same file.
    pub fn foundations_report_path(&self, base: &str) -> PathBuf {
        self.at.join("foundations").join(format!("{base}.txt"))
    }

    /// Where `base`'s own cached per-Check verdicts live —
    /// `scripts/land`'s `checks_on_the_base`'s own cache file, keyed by
    /// commit for the same reason.
    pub fn checks_cache_path(&self, base: &str) -> PathBuf {
        self.at.join("checks").join(format!("{base}.json"))
    }
}

/// The file name a branch is given under the state directory —
/// `hashlib.sha256(branch.encode()).hexdigest()[:16]`, bit for bit.
///
/// **Reversed from Stage 1's own hex-of-UTF-8-bytes encoding**, which this
/// module doc used to carry as "bijective... and no new dependency."
/// `scripts/test_land.py` recomputes this same key itself, independently of
/// this binary, to find a branch's outcome and log files on disk — so the
/// two encodings must agree exactly, not just each be internally
/// consistent. Sixteen hex characters is not bijective (a 64-bit space over
/// arbitrarily long names), but two branches sharing a key was already
/// `scripts/land`'s own accepted risk, carried forward rather than
/// improved on here. [`sha256_hex16`] is hand-rolled to keep the "no new
/// dependency" property while matching Python's own hash function exactly.
pub fn key(branch: &str) -> String {
    sha256_hex16(branch.as_bytes())
}

/// SHA-256 (FIPS 180-4), truncated to its first sixteen hex characters — the
/// first eight bytes, i.e. the first two words of the digest. Hand-rolled
/// rather than a `sha2` dependency, the way `queue::nonce` hand-rolls its
/// own entropy instead of pulling in `uuid`.
fn sha256_hex16(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let mut msg = data.to_vec();
    let bit_len = (data.len() as u64) * 8;
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for block in msg.chunks(64) {
        let mut w = [0u32; 64];
        for (i, word) in w.iter_mut().take(16).enumerate() {
            *word = u32::from_be_bytes([
                block[4 * i],
                block[4 * i + 1],
                block[4 * i + 2],
                block[4 * i + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    format!("{:08x}{:08x}", h[0], h[1])
}

#[cfg(test)]
mod sha256_tests {
    use super::sha256_hex16;

    /// The standard NIST test vector, truncated the same way `key` is —
    /// proof this hand-rolled digest is a real SHA-256 and not merely a
    /// stable one.
    #[test]
    fn matches_the_known_vector_for_abc() {
        assert_eq!(sha256_hex16(b"abc"), "ba7816bf8f01cfea");
    }

    #[test]
    fn matches_the_known_vector_for_the_empty_string() {
        assert_eq!(sha256_hex16(b""), "e3b0c44298fc1c14");
    }
}

fn git_common_dir(repo: &Path) -> Result<PathBuf, StateDirError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .output()
        .map_err(|cause| StateDirError::GitUnavailable {
            repo: repo.to_path_buf(),
            cause,
        })?;
    if !output.status.success() {
        return Err(StateDirError::NotARepository {
            repo: repo.to_path_buf(),
            why: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(PathBuf::from(
        String::from_utf8_lossy(&output.stdout).trim().to_string(),
    ))
}

/// Why a state directory could not be resolved.
#[derive(Debug)]
pub enum StateDirError {
    /// `git` itself could not be run — not installed, or `repo` does not
    /// exist.
    GitUnavailable {
        repo: PathBuf,
        cause: std::io::Error,
    },
    /// `repo` is not inside a git working tree.
    NotARepository { repo: PathBuf, why: String },
    /// One of the six subdirectories could not be created.
    SubdirectoryUnwritable {
        path: PathBuf,
        cause: std::io::Error,
    },
}

impl std::fmt::Display for StateDirError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateDirError::GitUnavailable { repo, .. } => {
                write!(out, "git could not be run against {}", repo.display())
            }
            StateDirError::NotARepository { repo, why } => {
                write!(out, "{} is not a git repository: {why}", repo.display())
            }
            StateDirError::SubdirectoryUnwritable { path, .. } => {
                write!(out, "{} could not be created", path.display())
            }
        }
    }
}

impl std::error::Error for StateDirError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StateDirError::GitUnavailable { cause, .. } => Some(cause),
            StateDirError::NotARepository { .. } => None,
            StateDirError::SubdirectoryUnwritable { cause, .. } => Some(cause),
        }
    }
}
