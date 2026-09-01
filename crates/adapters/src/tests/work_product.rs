//! What the diff says a Job has produced, asked of a real repository.
//!
//! These need git's own opinion — what a merge base is, whether a commit inside
//! a linked worktree is visible from the repository it belongs to, what
//! libgit2 counts as untracked. A fake here would assert this crate's guess at
//! each of those, and the guess about untracked directories has already been
//! wrong once (see [`repo::TempRepo::status_via_the_library`]).

use adapter_traits::{Change, LineCount, Vcs, WorkProduct, WorktreeSpec};

use crate::tests::repo::TempRepo;
use crate::worktree::GitVcs;

const JOB: &str = "01K3Q4R5S6T7V8W9X0Y1Z2A3B4";

/// A repository with one commit and a Job worktree cut from it.
fn worktree_for(repo: &TempRepo) -> adapter_traits::Worktree {
    let spec = WorktreeSpec::for_job(&repo.root_str(), JOB).expect("a legal spec");
    GitVcs::new().create_worktree(&spec).expect("a worktree")
}

#[test]
fn a_fresh_worktree_has_produced_nothing() {
    let repo = TempRepo::with_a_commit();
    let worktree = worktree_for(&repo);

    let changed = GitVcs::new().changed_files(&worktree).expect("a reading");
    assert!(
        changed.is_empty(),
        "a worktree nobody has worked in reported {:?}",
        changed.paths()
    );
}

#[test]
fn an_uncommitted_edit_counts() {
    let repo = TempRepo::with_a_commit();
    let worktree = worktree_for(&repo);
    std::fs::write(format!("{}/answer.txt", worktree.path()), "42").expect("the file");

    let changed = GitVcs::new().changed_files(&worktree).expect("a reading");
    assert_eq!(changed.paths(), ["answer.txt"]);
}

/// The case a diff against the worktree's own HEAD gets wrong: a Drone that
/// committed its work has produced the most, and would read as having produced
/// nothing.
#[test]
fn work_the_drone_committed_still_counts() {
    let repo = TempRepo::with_a_commit();
    let worktree = worktree_for(&repo);
    std::fs::write(format!("{}/answer.txt", worktree.path()), "42").expect("the file");
    commit_in(worktree.path(), "the drone's work");

    let changed = GitVcs::new().changed_files(&worktree).expect("a reading");
    assert_eq!(changed.paths(), ["answer.txt"]);
}

#[test]
fn a_deleted_file_counts() {
    let repo = TempRepo::with_a_commit();
    repo.write("doomed.txt", "here for now");
    repo.commit_everything("a file to delete");
    let worktree = worktree_for(&repo);
    std::fs::remove_file(format!("{}/doomed.txt", worktree.path())).expect("the removal");

    let changed = GitVcs::new().changed_files(&worktree).expect("a reading");
    assert_eq!(changed.paths(), ["doomed.txt"]);
}

/// **What the file list is for.** A person watching a Drone needs to tell a
/// file it wrote from one it deleted, and the kinds come from the same delta
/// walk the paths do — so git says which, and this crate does not guess.
///
/// The untracked file reads as **added** rather than as a kind of its own: the
/// diff asks for untracked precisely because an unstaged new file is work, and
/// reporting the staging would answer a question nobody asked.
#[test]
fn each_file_carries_what_happened_to_it() {
    let repo = TempRepo::with_a_commit();
    repo.write("doomed.txt", "here for now");
    repo.write("edited.txt", "before");
    repo.commit_everything("two files to work on");
    let worktree = worktree_for(&repo);
    std::fs::remove_file(format!("{}/doomed.txt", worktree.path())).expect("the removal");
    std::fs::write(format!("{}/edited.txt", worktree.path()), "after").expect("the edit");
    std::fs::write(format!("{}/written.txt", worktree.path()), "new").expect("the new file");

    let changed = GitVcs::new().changed_files(&worktree).expect("a reading");
    let mut seen: Vec<(&str, Change)> = changed
        .files()
        .iter()
        .map(|file| (file.path(), file.change()))
        .collect();
    seen.sort_by_key(|(path, _)| *path);
    assert_eq!(
        seen,
        vec![
            ("doomed.txt", Change::Deleted),
            ("edited.txt", Change::Modified),
            ("written.txt", Change::Added),
        ]
    );
}

/// **What the counted reading buys over the file list.** The same three files,
/// each with what it gained and lost — the numbers a finished Job's footprint
/// carries, and the ones the live seam deliberately does not.
///
/// A file written and never staged counts every line as added, for the reason
/// the module gives: an unstaged new file is work.
#[test]
fn each_counted_file_carries_the_lines_it_gained_and_lost() {
    let repo = TempRepo::with_a_commit();
    repo.write("doomed.txt", "one\ntwo\n");
    repo.write("edited.txt", "one\ntwo\nthree\n");
    repo.commit_everything("two files to work on");
    let worktree = worktree_for(&repo);
    std::fs::remove_file(format!("{}/doomed.txt", worktree.path())).expect("the removal");
    std::fs::write(
        format!("{}/edited.txt", worktree.path()),
        "one\nTWO\nthree\n",
    )
    .expect("the edit");
    std::fs::write(format!("{}/written.txt", worktree.path()), "new\nfile\n")
        .expect("the new file");

    let counted = GitVcs::new().counted_files(&worktree).expect("a reading");
    let mut seen: Vec<(&str, Change, Option<(u32, u32)>)> = counted
        .files()
        .iter()
        .map(|file| {
            (
                file.path(),
                file.change(),
                file.lines().map(|at| (at.added(), at.deleted())),
            )
        })
        .collect();
    seen.sort_by_key(|(path, _, _)| *path);
    assert_eq!(
        seen,
        vec![
            ("doomed.txt", Change::Deleted, Some((0, 2))),
            ("edited.txt", Change::Modified, Some((1, 1))),
            ("written.txt", Change::Added, Some((2, 0))),
        ]
    );
}

/// **Zero is a measurement and absent is not.** A file with nothing in it was
/// counted and the answer is `0` and `0`; a file nothing could count carries no
/// numbers at all. A reader that took one for the other would call an empty
/// file and a binary one the same thing.
#[test]
fn a_file_with_no_lines_in_it_counts_zero_rather_than_nothing() {
    let repo = TempRepo::with_a_commit();
    let worktree = worktree_for(&repo);
    std::fs::write(format!("{}/empty.txt", worktree.path()), "").expect("an empty new file");

    let counted = GitVcs::new().counted_files(&worktree).expect("a reading");
    let empty = counted
        .files()
        .iter()
        .find(|file| file.path() == "empty.txt")
        .expect("the empty file is in the reading");
    assert_eq!(empty.lines(), Some(LineCount::of(0, 0)));
}

/// A count is a reading like any other: it fails rather than answering an
/// empty list, for the reason [`changed_files`] does.
///
/// [`changed_files`]: adapter_traits::WorkProduct::changed_files
#[test]
fn a_worktree_that_is_not_there_cannot_be_counted_either() {
    let missing = adapter_traits::Worktree::at("/armada-no-such-worktree", "armada/nope");
    let read = GitVcs::new().counted_files(&missing);
    assert!(read.is_err(), "an unreadable worktree answered a count");
}

/// The main line moving on must not make a Job look busier than it is. The base
/// is the merge base, so a commit made on the repository's own branch after the
/// worktree was cut is not the Job's work.
#[test]
fn a_commit_on_the_main_line_afterwards_is_not_this_job_s_work() {
    let repo = TempRepo::with_a_commit();
    let worktree = worktree_for(&repo);
    repo.write("somebody-else.txt", "not the job's");
    repo.commit_everything("work outside the Job");

    let changed = GitVcs::new().changed_files(&worktree).expect("a reading");
    assert!(
        changed.is_empty(),
        "the Job was credited with {:?}",
        changed.paths()
    );
}

#[test]
fn a_worktree_that_is_not_there_is_an_error_and_not_an_empty_diff() {
    let missing = adapter_traits::Worktree::at("/armada-no-such-worktree", "armada/nope");
    let read = GitVcs::new().changed_files(&missing);
    assert!(
        read.is_err(),
        "an unreadable worktree answered {:?}",
        read.map(|changed| changed.paths().to_vec())
    );
}

/// Commit whatever is in a worktree, from inside that worktree.
fn commit_in(path: &str, message: &str) {
    let repo = git2::Repository::open(path).expect("the worktree repository");
    let mut index = repo.index().expect("the index");
    index
        .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
        .expect("staged everything");
    index.write().expect("the index written");
    let tree = repo
        .find_tree(index.write_tree().expect("a tree"))
        .expect("the tree");
    let who = git2::Signature::now("armada", "armada@example.invalid").expect("a signature");
    let parent = repo
        .head()
        .and_then(|head| head.peel_to_commit())
        .expect("a parent commit");
    repo.commit(Some("HEAD"), &who, &who, message, &tree, &[&parent])
        .expect("a commit");
}

/// The rendering follows the file list. A Drone that wrote a file and never
/// staged it has produced work — this module says so of the count, and a patch
/// that printed a header and no lines would tell the Judge otherwise.
#[test]
fn a_file_the_drone_never_staged_reaches_the_judge_with_its_contents() {
    let repo = TempRepo::with_a_commit();
    let worktree = worktree_for(&repo);
    std::fs::write(format!("{}/answer.txt", worktree.path()), "42\n").expect("the file");

    let patch = GitVcs::new().patch(&worktree).expect("a reading");
    assert!(patch.as_str().contains("+42"), "{}", patch.as_str());
}
