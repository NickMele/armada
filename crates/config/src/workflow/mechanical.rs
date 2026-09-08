//! `mechanical_checks` on a step: the deterministic tier, read off a file.
//!
//! **Nothing here knows what a step is.** An entry is read from one list item,
//! and the single rule needing the whole list — a step delivers one file — is
//! [`checks`]'s. Whether a `check` name resolves is not asked here either: it
//! is asked once, against a Manifest, by [`crate::ResolvedWorkflow`].

use core_model::{ARTIFACT_EXISTS, DIFF_NONEMPTY, MANIFEST_CHECK};
use serde_yaml_ng::Value;

use crate::error::{BadTarget, Fault, Refusal};
use crate::yaml::{self, Table};

/// A deterministic assertion. **Three types, of five sanctioned.**
///
/// `test_run` and `pr_merged` are the other two, and each needs machinery M1
/// has not built — a per-step test invocation distinct from a named Check, and
/// a merged pull request.
///
/// **`artifact_exists` is carried now, and it carries a path rather than a
/// name.** The schema's samples name a registry entry — `root_cause_note` —
/// and there is no artifact registry to resolve one against. What there is, on
/// every step, is a worktree Fleet reads for itself, so the target is the
/// worktree-relative path of the file the step writes. That is also what makes
/// a step's product something the next step can open rather than a sentence the
/// last Drone typed about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MechanicalCheck {
    /// A named Check in the repo's `armada.yml` ran and exited as expected.
    ///
    /// `check` is a name, never a command string, so what actually runs is
    /// written in exactly one place. Whether that name exists is not this
    /// type's question — see [`crate::ResolvedWorkflow`].
    ManifestCheck {
        check: String,
        expect_exit_code: i64,
    },
    /// The step produced a non-empty diff.
    DiffNonempty,
    /// The step wrote the file named by `target`.
    ArtifactExists { target: String },
}

const CHECK_TYPE_LEGAL: &[&str] = &[
    MANIFEST_CHECK,
    DIFF_NONEMPTY,
    ARTIFACT_EXISTS,
    "test_run",
    "pr_merged",
];
const CHECK_TYPE_M1: &[&str] = &[MANIFEST_CHECK, DIFF_NONEMPTY, ARTIFACT_EXISTS];
const MANIFEST_CHECK_KEYS: &[&str] = &["type", "check", "expect_exit_code"];
const DIFF_NONEMPTY_KEYS: &[&str] = &["type"];
const ARTIFACT_EXISTS_KEYS: &[&str] = &["type", "target"];

/// Every mechanical check a step declares, in file order.
///
/// An absent key and `mechanical_checks: []` are the same empty list, which is
/// what a gateless step wrote either way.
pub(crate) fn checks(table: &mut Table<'_>, out: &mut Vec<Refusal>) -> Vec<MechanicalCheck> {
    table
        .optional("mechanical_checks")
        .and_then(|value| yaml::list(&table.at("mechanical_checks"), value, out))
        .map(|items| {
            // **The second `artifact_exists` is refused, not dropped.** A step
            // has one deliverable — Fleet reads it into the Judge's brief as
            // the document the step produced — so two would be a choice made by
            // whichever the reader happened to reach first.
            let mut delivers: Option<String> = None;
            let mut checks = Vec::with_capacity(items.len());
            for (at, item) in items.iter() {
                let Some(check) = check(at, item, out) else {
                    continue;
                };
                if let MechanicalCheck::ArtifactExists { target } = &check {
                    match &delivers {
                        None => delivers = Some(target.clone()),
                        Some(first) => {
                            out.push(Refusal::new(
                                format!("{at}.target"),
                                Fault::TwoDeliverables {
                                    first: first.clone(),
                                },
                            ));
                            continue;
                        }
                    }
                }
                checks.push(check);
            }
            checks
        })
        .unwrap_or_default()
}

/// A `target` that names exactly one file inside the worktree, or a refusal
/// saying which of the four ways it does not.
///
/// **Every one of these is a step no Drone could ever pass**, and each was
/// cheaper to catch here than at the gate. v1 measured the glob case on a real
/// Job: the `design` workflow named `docs/design/*.md`, the gate probed it as a
/// literal path, and the step retried until the Job hit its token ceiling with
/// the file it asked for sitting on disk. A pattern is refused rather than
/// matched because Fleet has to hand the next step's Drone a path, and
/// "whichever file matched" is not one.
///
/// The path is not resolved against a worktree here. This crate reads files and
/// never a Job's, and whether the Drone wrote it is the gate's question — what
/// is answered here is whether the string could name it at all.
fn artifact_target(at: &str, target: String, out: &mut Vec<Refusal>) -> Option<String> {
    let why = if target.contains('*') || target.contains('?') {
        Some(BadTarget::Globbed)
    } else if target.starts_with('/') {
        Some(BadTarget::Absolute)
    } else if target.ends_with('/') {
        Some(BadTarget::ADirectory)
    } else if target.split('/').any(|segment| segment == "..") {
        Some(BadTarget::Escapes)
    } else {
        None
    };
    match why {
        None => Some(target),
        Some(why) => {
            out.push(Refusal::new(
                at,
                Fault::NotAnArtifactPath { value: target, why },
            ));
            None
        }
    }
}

fn check(at: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<MechanicalCheck> {
    let mut table = Table::open(at, value, out)?;
    let kind = table
        .required("type", out)
        .and_then(|value| yaml::text(&table.at("type"), value, out))?;

    match kind.as_str() {
        MANIFEST_CHECK => {
            let check = table
                .required("check", out)
                .and_then(|value| yaml::text(&table.at("check"), value, out));
            let expect_exit_code = table
                .required("expect_exit_code", out)
                .and_then(|value| yaml::integer(&table.at("expect_exit_code"), value, out));
            table.close(MANIFEST_CHECK_KEYS, out);
            Some(MechanicalCheck::ManifestCheck {
                check: check?,
                expect_exit_code: expect_exit_code?,
            })
        }
        DIFF_NONEMPTY => {
            table.close(DIFF_NONEMPTY_KEYS, out);
            Some(MechanicalCheck::DiffNonempty)
        }
        ARTIFACT_EXISTS => {
            let target_key = table.at("target");
            let target = table
                .required("target", out)
                .and_then(|value| yaml::text(&target_key, value, out))
                .and_then(|target| artifact_target(&target_key, target, out));
            table.close(ARTIFACT_EXISTS_KEYS, out);
            Some(MechanicalCheck::ArtifactExists { target: target? })
        }
        other => {
            let fault = if CHECK_TYPE_LEGAL.contains(&other) {
                Fault::NotYetCarried {
                    value: kind.clone(),
                    carried: CHECK_TYPE_M1,
                }
            } else {
                Fault::NotInTheSchema {
                    value: kind.clone(),
                    legal: CHECK_TYPE_LEGAL,
                }
            };
            out.push(Refusal::new(table.at("type"), fault));
            None
        }
    }
}
