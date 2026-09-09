//! `mechanical_checks` on a step: the deterministic tier, read off a file.
//!
//! **Nothing here knows what a step is.** An entry is read from one list item,
//! and the two rules needing the whole list — a step delivers one file, and a
//! step names Checks or gates on all of them — are [`checks`]'s. Whether a
//! `check` name resolves is not asked here either: it is asked once, against a
//! Manifest, by [`crate::ResolvedWorkflow`].
//!
//! **Nothing here has a Manifest, which is what fixes where each half lands.**
//! `every_manifest_check` parses to a variant that names no Check, and
//! `expect_exit_code` parses to what the step wrote or to nothing; both are
//! answered against `armada.yml` one layer up, where the Checks exist. So this
//! file gained a type and no lookup.

use core_model::{ARTIFACT_EXISTS, DIFF_NONEMPTY, MANIFEST_CHECK};
use serde_yaml_ng::Value;

use crate::error::{BadTarget, Fault, Refusal};
use crate::yaml::{self, Table};

/// A deterministic assertion. **Four types, of six sanctioned.**
///
/// `test_run` and `pr_merged` are the two not carried, and each needs machinery
/// M1 has not built — a per-step test invocation distinct from a named Check,
/// and a merged pull request.
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
        /// **`None` on a step that leaves the key out, which is the shape to
        /// write.** The expectation is the Check's own and lives in
        /// `armada.yml`; this survives only because seven shipped workflows
        /// spell it and a change that stopped parsing them would stop every
        /// Job in flight. Where a step does write it, it must say what the
        /// Manifest says — [`crate::ResolvedWorkflow`] refuses the pair that
        /// disagrees, so this is never a second answer.
        expect_exit_code: Option<i64>,
    },
    /// Every Check the repo's `armada.yml` declares, whichever those are.
    ///
    /// **The point is that a shipped workflow stops enumerating.** A step
    /// listing `build`, `format`, `typecheck` and four more is a step that fits
    /// one repository and goes stale the moment that repository declares an
    /// eighth Check — which is a gate that quietly stopped covering something,
    /// the failure `when` exists one file along to prevent.
    ///
    /// **A step says this; a step never omits it into meaning.** An empty
    /// `mechanical_checks` is an ungated step and stays one: half the shipped
    /// workflows have steps that produce a document nothing compiles, and
    /// reading absence as *all* would gate every one of them on this
    /// repository's Rust build.
    ///
    /// It resolves to one [`core_model::ResolvedCheck::ManifestCheck`] per
    /// declared Check and never travels further than that, which is why there
    /// is no variant for it on the record and nothing in `store` or on the wire
    /// to teach: a Job still freezes the list of Checks it actually gated on,
    /// and an `armada.yml` that gains one afterwards changes the next Job.
    EveryManifestCheck,
    /// The step produced a non-empty diff.
    DiffNonempty,
    /// The step wrote the file named by `target`.
    ArtifactExists { target: String },
}

/// The schema's `type` value for *every declared Check*.
///
/// **Spelled here and not in `core-model` beside the other three**, which is
/// where a `type` value goes when the record can carry it. This one cannot
/// reach a record: it is expanded against a Manifest during resolution, so
/// `store` and the wire never see the word. A constant in `core-model` would
/// advertise a variant of `ResolvedCheck` that does not exist.
const EVERY_MANIFEST_CHECK: &str = "every_manifest_check";

const CHECK_TYPE_LEGAL: &[&str] = &[
    MANIFEST_CHECK,
    EVERY_MANIFEST_CHECK,
    DIFF_NONEMPTY,
    ARTIFACT_EXISTS,
    "test_run",
    "pr_merged",
];
const CHECK_TYPE_M1: &[&str] = &[
    MANIFEST_CHECK,
    EVERY_MANIFEST_CHECK,
    DIFF_NONEMPTY,
    ARTIFACT_EXISTS,
];
const MANIFEST_CHECK_KEYS: &[&str] = &["type", "check", "expect_exit_code"];
/// **`type` and nothing else**, which is the whole of the spelling: the set is
/// the Manifest's to state and there is no key here to trim it with. A step
/// that wants fewer names them one at a time, as every step did before.
const EVERY_MANIFEST_CHECK_KEYS: &[&str] = &["type"];
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
            // Where the step first said something about the Manifest's Checks,
            // by each spelling. A step names Checks or gates on all of them,
            // never both — `Fault::EveryCheckAndByName` says why, and the
            // second half of the pair is refused where it is written.
            let mut every_at: Option<usize> = None;
            let mut named_at: Option<usize> = None;
            let mut checks = Vec::with_capacity(items.len());
            for (n, (at, item)) in items.iter().enumerate() {
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
                // A second, third and fourth `manifest_check` is ordinary — a
                // step may gate on `build` and on `test`. What cannot stand is
                // a pair with `every_manifest_check` on either side of it, or
                // two of those.
                let clash = match &check {
                    MechanicalCheck::EveryManifestCheck => every_at.or(named_at),
                    MechanicalCheck::ManifestCheck { .. } => every_at,
                    _ => None,
                };
                match (clash, &check) {
                    (Some(first_at), _) => {
                        out.push(Refusal::new(
                            format!("{at}.type"),
                            Fault::EveryCheckAndByName { first_at },
                        ));
                        continue;
                    }
                    (None, MechanicalCheck::EveryManifestCheck) => every_at = Some(n),
                    (None, MechanicalCheck::ManifestCheck { .. }) => {
                        named_at.get_or_insert(n);
                    }
                    (None, _) => {}
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
            // **Optional now, and absent is not zero — it is *the Manifest
            // says*.** The two readings are opposite where a Check declares
            // its own non-zero code, so absence is carried as absence all the
            // way to resolution rather than filled in here. A step that does
            // write it is held to agreeing with `armada.yml`, which is a
            // question this parser has no Manifest to ask.
            let expect_exit_code = table
                .optional("expect_exit_code")
                .and_then(|value| yaml::integer(&table.at("expect_exit_code"), value, out));
            table.close(MANIFEST_CHECK_KEYS, out);
            Some(MechanicalCheck::ManifestCheck {
                check: check?,
                expect_exit_code,
            })
        }
        EVERY_MANIFEST_CHECK => {
            table.close(EVERY_MANIFEST_CHECK_KEYS, out);
            Some(MechanicalCheck::EveryManifestCheck)
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
