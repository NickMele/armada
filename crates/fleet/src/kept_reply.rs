//! Where a proposal's two replies are kept, once asking once more still did
//! not read either one.
//!
//! `#831`: a refusal used to be checked only against the code that read the
//! reply, never against the reply itself, because nothing kept it. Neither
//! reply reaches the wire — `crate::refusing` puts the path this writes on the
//! refusal instead, so a person can still go and read what the model said.
//!
//! `<records_root>/.armada/proposals/`, beside `asked::briefs_dir` — never the
//! repository, and never part of `crate::records::migrating`'s list: nothing
//! wrote a proposal's reply anywhere before this existed, so there is nothing
//! under a repository's own `.armada/` to move.

use std::io::Write;
use std::path::{Path, PathBuf};

/// Where every proposal's kept replies live, under this repository's own
/// share of Fleet's data directory.
pub(crate) fn proposals_dir(records_root: &str) -> PathBuf {
    Path::new(records_root).join(".armada").join("proposals")
}

/// Write both replies down, and answer with the path to put on the refusal —
/// relative to `records_root`, the same shape [`crate::asked::Asked::kept`]
/// answers with.
///
/// **`None` is ordinary and never an error**, on that function's own rule: a
/// directory that will not open or a disk that refused is not a reason to
/// fail a refusal that is already in flight. What is lost is the re-read, and
/// the absent field is how a person is told there is nowhere to look.
pub(crate) fn kept(
    records_root: &str,
    proposal_id: &str,
    first_reply: &str,
    second_reply: &str,
) -> Option<String> {
    let dir = proposals_dir(records_root);
    std::fs::create_dir_all(&dir).ok()?;
    let mut file = std::fs::File::create(dir.join(format!("{proposal_id}.txt"))).ok()?;
    write!(
        file,
        "first reply:\n{first_reply}\n\nsecond reply:\n{second_reply}\n"
    )
    .ok()?;
    Some(format!(".armada/proposals/{proposal_id}.txt"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_replies_are_read_back_from_the_path_this_answers_with() {
        let home = crate::tests::tmp::TempDir::new();
        let records_root = home.path().to_string_lossy().to_string();

        let path = kept(
            &records_root,
            "01PROPOSAL",
            "the first answer",
            "the second answer",
        )
        .expect("the directory is fresh and writable");

        assert_eq!(path, ".armada/proposals/01PROPOSAL.txt");
        let written = std::fs::read_to_string(Path::new(&records_root).join(&path))
            .expect("the file this path names");
        assert!(written.contains("the first answer"));
        assert!(written.contains("the second answer"));
    }
}
