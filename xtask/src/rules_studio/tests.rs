//! The readers, against the shapes they read.
//!
//! Each reader answers a set, and an empty answer is what
//! [`super::same_set`] refuses — so what these prove is that the shapes in the
//! tree are read at all, and that a shape one step off is not read as agreement.

use super::{registry, sources};
use crate::Report;

const ENUM: &str = r#"
spelled! {
    /// What a node is.
    StudioNodeKind {
        Run => "run",
        /// A note.
        Note => "note",
        Job => "job",
    }
}

impl StudioNodeKind {
    pub fn states(&self) -> &'static [StudioNodeState] {
        use StudioNodeState as S;
        match self {
            // Their state is the run's or the Job's own.
            StudioNodeKind::Run | StudioNodeKind::Job => &[],
            StudioNodeKind::Note => &[
                S::Draft,
                S::Frozen,
            ],
        }
    }

    pub fn added_by_hand(&self) -> bool {
        matches!(self, StudioNodeKind::Note)
    }
}
"#;

#[test]
fn an_enum_reads_back_as_its_wire_spellings() {
    let read = sources::spelled(ENUM, "StudioNodeKind");
    assert_eq!(read.len(), 3);
    assert_eq!(read.get("pull_request"), None);
    assert_eq!(read.get("note").map(String::as_str), Some("Note"));
}

#[test]
fn an_arm_naming_several_kinds_answers_for_each_of_them() {
    let read = sources::states_by_kind(ENUM);
    assert_eq!(read.get("Run"), Some(&Vec::new()));
    assert_eq!(read.get("Job"), Some(&Vec::new()));
    assert_eq!(
        read.get("Note").map(Vec::as_slice),
        Some(["Draft".to_string(), "Frozen".to_string()].as_slice())
    );
}

#[test]
fn added_by_hand_is_read_from_its_own_body() {
    let read = sources::added_by_hand(ENUM);
    assert_eq!(read.len(), 1);
    assert!(read.contains("Note"));
}

const MIGRATIONS: &str = r##"
pub(crate) const V77: &str = r#"
CREATE TABLE studio_nodes (
    kind TEXT NOT NULL CHECK (kind IN ('run', 'note')),
    state TEXT CHECK (state IS NULL OR state IN ('draft'))
) STRICT;
"#;

pub(crate) const V79: &str = r#"
CREATE TABLE studio_nodes (
    kind TEXT NOT NULL CHECK (kind IN ('run', 'note', 'job')),
    state TEXT CHECK (state IS NULL OR state IN ('draft', 'frozen'))
) STRICT;

CREATE TABLE studio_edges (
    kind TEXT NOT NULL CHECK (kind IN ('produced', 'blocks')),
    CHECK (from_node <> to_node),
    CHECK (kind <> 'produced' OR standing = 'accepted')
) STRICT;
"#;
"##;

#[test]
fn the_check_read_is_the_last_migrations_and_not_the_first() {
    let kinds = sources::check_set(MIGRATIONS, "studio_nodes", "kind");
    assert!(kinds.contains("job"), "V77's narrower set was read instead");
    assert_eq!(kinds.len(), 3);
    let states = sources::check_set(MIGRATIONS, "studio_nodes", "state");
    assert_eq!(states.len(), 2);
}

#[test]
fn the_edge_the_store_refuses_to_leave_proposed_is_named() {
    let read = sources::accepted_on_sight(MIGRATIONS);
    assert_eq!(read.len(), 1);
    assert!(read.contains("produced"));
}

const WIRE: &str = r#"
export type StudioNodeContent =
  /** A reference to the run; never its status. */
  | { kind: "run"; run_id: string }
  /**
   * What a person pointed at and said; fixed at capture.
   */
  | { kind: "note"; said: string }
  | { kind: "job"; job_id: string };

export type ForgeState = "open" | "closed";

export type ProposeStudioEdge = {
  from: string;
  to: string;
  kind: "same_as" | "blocks";
};
"#;

#[test]
fn a_union_of_tags_reads_back_as_its_tags() {
    let read = sources::tags(WIRE, "StudioNodeContent");
    assert_eq!(
        read.len(),
        3,
        "a doc comment's own `;` ended the union early"
    );
    assert!(read.contains("job"));
}

#[test]
fn an_object_type_is_read_past_its_first_semicolon() {
    let read = sources::literals(WIRE, "ProposeStudioEdge");
    assert_eq!(
        read.len(),
        2,
        "the `from` field ended the declaration early"
    );
    assert!(read.contains("same_as"));
    assert_eq!(sources::literals(WIRE, "ForgeState").len(), 2);
}

#[test]
fn the_lexicon_bans_the_word_and_not_the_article() {
    let read = sources::never_words(
        "### Lexicon\n\n- **Job** one unit of work. Never run, ticket.\n- **Bridge** the\n  \
         operational surfaces. Never the dashboard, the UI.\n\n### Retired terms\n",
    );
    assert!(read.contains("ticket"));
    assert!(read.contains("dashboard"), "`the ` was not stripped");
    assert!(read.contains("ui"));
    assert!(!read.contains("the dashboard"));
}

#[test]
fn a_field_above_every_table_belongs_to_no_row_and_is_named() {
    let mut report = Report::new("test");
    let rows = registry::rows("name = \"Run\"\n", "x.toml", &mut report);
    assert!(rows.is_empty());
    assert!(report.failed());
}

#[test]
fn a_hash_inside_prose_is_not_a_comment() {
    let mut report = Report::new("test");
    let rows = registry::rows(
        "[nodes.run]\nname = \"Run\"\nstates = []\nstatus_colour = true\nnotes = \"See #1313\"\n",
        "x.toml",
        &mut report,
    );
    assert!(!report.failed());
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].strings.get("notes").map(String::as_str),
        Some("See #1313")
    );
    assert_eq!(rows[0].bools.get("status_colour"), Some(&true));
    assert_eq!(rows[0].arrays.get("states").map(Vec::len), Some(0));
}

/// The whole rule, over the tree it gates — so a set that drifts is caught by
/// `cargo nextest run -p xtask` and not only by the gate.
#[test]
fn the_repository_holds_its_own_studio_kinds() {
    let report = super::the_kinds_are_one_set_everywhere(&crate::repo_root());
    let complaints: Vec<&String> = report
        .findings
        .iter()
        .map(|f| match f {
            crate::Finding::Fail(what) | crate::Finding::Warn(what) => what,
        })
        .collect();
    assert!(complaints.is_empty(), "{complaints:#?}");
}
