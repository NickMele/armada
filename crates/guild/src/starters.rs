//! The starter content `guild init` copies out of `templates/guild/`, and
//! never touches again.
//!
//! | Starter | Why it ships | Source |
//! |---|---|---|
//! | Four workflows | M4's loop has nothing to run without them (`PLAN.md` §14.6) | `templates/guild/workflows/` |
//! | `onboard-repo` | The guild's first real content, and the loop that writes a repo's `armada.yml` with you (§13.4) | `templates/guild/skills/onboard-repo/` |
//! | `helm.md` | Without it M3 has an orchestrator with no persona to run (§15.4) | `templates/guild/subagents/` |
//! | `permissions.yml` | What a Drone may do unattended — shipped written out so the posture is readable rather than compiled in ([`crate::permissions`]) | `templates/guild/` |
//!
//! **`permissions.yml` is the one starter Armada also has a copy of in code.**
//! [`armada_core::fleet::drone::Posture::default`] is what a guild without the
//! file gets, and `permissions::tests` holds the two against each other so they
//! cannot drift.
//!
//! # Copied, not confirmed
//!
//! `PLAN.md` §13.4 settles this: *"A confirmation step on a file you have not
//! read yet is theatre."* The starters are written, the summary says they were
//! written, and `armada guild edit` changes them once you have an opinion.
//!
//! # Embedded rather than read from disk
//!
//! `include_str!` at build time, so a `guild init` run from an installed binary
//! finds them — there is no `templates/` next to `~/.local/bin/armada`, and a
//! starter set that only works from a checkout is a starter set that works for
//! nobody who did not build the tool.
//!
//! **The consequence is that `templates/guild/` is a compile-time input.** A
//! syntax error in a starter workflow does not fail at run time on somebody's
//! machine; the test at the bottom of this file parses every one of them, so it
//! fails in the gate.

use crate::interview::{Answers, Ceilings};
use std::path::Path;

/// One file to write into a fresh guild.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Starter {
    /// Where it lands, relative to `~/.armada/guild/`.
    pub path: &'static str,
    /// Its content, embedded at build time.
    pub body: &'static str,
}

/// The starter workflows (`PLAN.md` §14.6).
///
/// **Four a person spawns and one Fleet spawns.** `review` is not one of the
/// four labels classification returns ([`armada_core::fleet::workflow::STARTERS`]);
/// it ships because a `review_clean` step is satisfied by Fleet starting a Job
/// to run it, and a guild without the file is a guild where `bug` and `feature`
/// both stop at their review step.
pub const WORKFLOWS: [Starter; 5] = [
    Starter {
        path: "workflows/design.yml",
        body: include_str!("../../../templates/guild/workflows/design.yml"),
    },
    Starter {
        path: "workflows/plan.yml",
        body: include_str!("../../../templates/guild/workflows/plan.yml"),
    },
    Starter {
        path: "workflows/feature.yml",
        body: include_str!("../../../templates/guild/workflows/feature.yml"),
    },
    Starter {
        path: "workflows/bug.yml",
        body: include_str!("../../../templates/guild/workflows/bug.yml"),
    },
    Starter {
        path: "workflows/review.yml",
        body: include_str!("../../../templates/guild/workflows/review.yml"),
    },
];

/// Everything else `guild init` copies: the schema the workflows are checked
/// against, the starter skill, the starter persona, and the Drone permission
/// posture.
pub const OTHERS: [Starter; 5] = [
    Starter {
        path: crate::permissions::FILE,
        body: include_str!("../../../templates/guild/permissions.yml"),
    },
    Starter {
        path: "workflows/workflow.schema.json",
        body: include_str!("../../../templates/guild/workflows/workflow.schema.json"),
    },
    Starter {
        path: "skills/onboard-repo/SKILL.md",
        body: include_str!("../../../templates/guild/skills/onboard-repo/SKILL.md"),
    },
    // The skill the `review` workflow's one step names. It ships beside the
    // workflow rather than being left to the repository, because a
    // `review_clean` gate is Fleet's to satisfy on every machine — a reviewer
    // whose skill does not resolve is one that reviews whatever it feels like.
    Starter {
        path: "skills/review-diff/SKILL.md",
        body: include_str!("../../../templates/guild/skills/review-diff/SKILL.md"),
    },
    Starter {
        path: "subagents/helm.md",
        body: include_str!("../../../templates/guild/subagents/helm.md"),
    },
];

/// Every starter, in the order they are written.
pub fn all() -> Vec<Starter> {
    WORKFLOWS.iter().chain(OTHERS.iter()).copied().collect()
}

/// The starter at a guild-relative path.
///
/// **By name rather than by index.** Two tests below reached into `OTHERS[0]`
/// for the workflow schema, and adding a fourth starter at the front of the
/// list made both of them assert about a different file — which they did
/// notice, but only because the new file happened not to be JSON.
pub fn starter(path: &str) -> Option<Starter> {
    all().into_iter().find(|starter| starter.path == path)
}

/// The workflow name a starter path belongs to — `workflows/bug.yml` is `bug`.
fn workflow_name(path: &str) -> Option<&str> {
    path.strip_prefix("workflows/")?.strip_suffix(".yml")
}

/// Write the starters into a guild, applying the interview's ceilings.
///
/// **Question 4 is the only one that reaches a starter**, and it reaches it by
/// rewriting three lines of an already-written file rather than by templating
/// it. The starters are meant to be read as finished files with their reasoning
/// in them; a `{{ iterations }}` in the middle of that turns a document you can
/// edit into a document you can only regenerate.
pub fn write(guild_root: &Path, answers: &Answers) -> std::io::Result<Vec<String>> {
    let mut written = Vec::new();
    for starter in all() {
        let body = match workflow_name(starter.path) {
            Some(name) => with_ceilings(starter.body, answers.ceilings_for(name)),
            None => starter.body.to_string(),
        };
        let path = guild_root.join(starter.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, body)?;
        written.push(starter.path.to_string());
    }

    // **The provenance stamp, written last and written every time.** It is not
    // one of the starters because it is generated rather than shipped — it
    // records the digest of the set just written — and without it a guild has
    // no base, which is the whole of `docs/reserved/006`: not a missing merge
    // engine, a missing record of where the files came from.
    std::fs::write(
        guild_root.join(crate::upstream::STAMP),
        crate::upstream::stamp(),
    )?;
    written.push(crate::upstream::STAMP.to_string());

    Ok(written)
}

/// The same file with its three budget numbers replaced.
///
/// Line-oriented and keyed on the exact indented spelling the starters use, so
/// a workflow the user has since restructured is not silently rewritten in some
/// other place that happens to say `attempts:`.
fn with_ceilings(body: &str, ceilings: Ceilings) -> String {
    let mut out = String::with_capacity(body.len());
    for line in body.lines() {
        let replaced = match line.trim_start() {
            l if l.starts_with("attempts:") => Some(format!("  attempts: {}", ceilings.attempts)),
            l if l.starts_with("cost:") => Some(format!("  cost: {:.2}", ceilings.cost_usd)),
            l if l.starts_with("wall_clock:") => {
                Some(format!("  wall_clock: {}m", ceilings.wall_clock_minutes))
            }
            _ => None,
        };
        out.push_str(replaced.as_deref().unwrap_or(line));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The workflow schema, **found by name**. See [`starter`].
    fn schema_body() -> &'static str {
        starter("workflows/workflow.schema.json")
            .expect("the schema is shipped")
            .body
    }

    /// **Every starter PHASES.md §8.4 names, present and non-empty.** The list
    /// is what `guild init` copies; a starter dropped from it is a milestone
    /// that ships an orchestrator with no persona or a loop with nothing to
    /// run, and neither fails loudly.
    #[test]
    fn every_starter_the_milestone_names_is_embedded() {
        let paths: Vec<&str> = all().iter().map(|s| s.path).collect();
        for expected in [
            "workflows/design.yml",
            "workflows/plan.yml",
            "workflows/feature.yml",
            "workflows/bug.yml",
            "workflows/workflow.schema.json",
            "skills/onboard-repo/SKILL.md",
            "subagents/helm.md",
            "permissions.yml",
        ] {
            assert!(paths.contains(&expected), "`{expected}` is not shipped");
        }
        for starter in all() {
            assert!(
                starter.body.len() > 200,
                "`{}` is embedded but nearly empty",
                starter.path
            );
        }
    }

    /// A starter that does not parse is a `guild init` that half-succeeds on
    /// somebody's machine. `include_str!` makes them a compile-time input, and
    /// this makes them a gate-time one.
    #[test]
    fn every_starter_workflow_parses_and_declares_the_keys_the_schema_requires() {
        for starter in WORKFLOWS {
            let parsed: serde_yaml_ng::Value = serde_yaml_ng::from_str(starter.body)
                .unwrap_or_else(|e| panic!("{} does not parse: {e}", starter.path));
            for key in ["name", "description", "ends_at", "budget", "steps"] {
                assert!(
                    parsed.get(key).is_some(),
                    "{} has no `{key}:`",
                    starter.path
                );
            }
            let budget = parsed.get("budget").unwrap();
            for key in ["attempts", "cost", "wall_clock", "on_exhausted"] {
                assert!(
                    budget.get(key).is_some(),
                    "{} has no `budget.{key}:`",
                    starter.path
                );
            }
            let steps = parsed.get("steps").and_then(|s| s.as_sequence()).unwrap();
            assert!(!steps.is_empty(), "{} has no steps", starter.path);
            for step in steps {
                assert!(
                    step.get("id").is_some(),
                    "a step of {} has no id",
                    starter.path
                );
                let verify = step
                    .get("verify")
                    .unwrap_or_else(|| panic!("a step of {} has no verify", starter.path));
                assert!(verify.get("must").is_some());
                // **At most one of `skill:` and `workflow:`.** Two is ambiguous
                // about who runs the step, and the schema rejects it.
                assert!(
                    !(step.get("skill").is_some() && step.get("workflow").is_some()),
                    "a step of {} names both a skill and a workflow",
                    starter.path
                );
            }
        }
    }

    /// The schema is itself valid JSON, and it is the file `guild verify` will
    /// be pointed at.
    #[test]
    fn the_workflow_schema_is_valid_json_and_lists_every_predicate() {
        let schema: serde_json::Value = serde_json::from_str(schema_body()).unwrap();
        let predicates = schema["$defs"]["predicate"]["enum"].as_array().unwrap();
        for predicate in [
            "always",
            "check_passes",
            "failing_test_exists",
            "artifact_exists",
            "review_clean",
            "human_approves",
            "branch_exists",
            "subjob_passed",
        ] {
            assert!(
                predicates.iter().any(|p| p == predicate),
                "`{predicate}` is in PLAN.md §14.6 and not in the schema"
            );
        }
    }

    /// **Every predicate a starter uses is one the schema allows.** The two
    /// drifting apart is how a shipped workflow stops validating against a
    /// shipped schema, which nothing else would catch.
    #[test]
    fn no_starter_uses_a_predicate_the_schema_does_not_define() {
        let schema: serde_json::Value = serde_json::from_str(schema_body()).unwrap();
        let predicates = schema["$defs"]["predicate"]["enum"].as_array().unwrap();
        for starter in WORKFLOWS {
            let parsed: serde_yaml_ng::Value = serde_yaml_ng::from_str(starter.body).unwrap();
            for step in parsed["steps"].as_sequence().unwrap() {
                let must = step["verify"]["must"].as_str().unwrap();
                assert!(
                    predicates.iter().any(|p| p == must),
                    "{} uses `{must}`, which the schema does not define",
                    starter.path
                );
            }
        }
    }

    /// The defaults land as §14.6 specifies them: the advisory triple on the
    /// two workflows that end at you, the autonomous one on the two that close.
    #[test]
    fn a_defaulted_interview_writes_the_per_workflow_ceilings() {
        let guild = tempfile::tempdir().unwrap();
        write(guild.path(), &Answers::all_defaulted()).unwrap();

        let plan = std::fs::read_to_string(guild.path().join("workflows/plan.yml")).unwrap();
        assert!(plan.contains("attempts: 3"), "{plan}");
        assert!(plan.contains("cost: 5.00"), "{plan}");

        let bug = std::fs::read_to_string(guild.path().join("workflows/bug.yml")).unwrap();
        assert!(bug.contains("attempts: 3"), "{bug}");
        assert!(bug.contains("cost: 10.00"), "{bug}");
        assert!(bug.contains("wall_clock: 90m"), "{bug}");
    }

    /// Answered questions 4–6 replace them everywhere, and the file is still a
    /// file — the prose and the comments survive.
    #[test]
    fn an_answered_ceiling_rewrites_the_numbers_and_leaves_the_document_alone() {
        let guild = tempfile::tempdir().unwrap();
        let answers = Answers {
            attempts: Some(8),
            cost_usd: Some(7.50),
            wall_clock_minutes: Some(30),
            ..Answers::default()
        };
        write(guild.path(), &answers).unwrap();

        for name in ["design", "plan", "feature", "bug"] {
            let body = std::fs::read_to_string(guild.path().join(format!("workflows/{name}.yml")))
                .unwrap();
            assert!(body.contains("attempts: 8"), "{name}: {body}");
            assert!(body.contains("cost: 7.50"), "{name}: {body}");
            assert!(body.contains("wall_clock: 30m"), "{name}: {body}");
            assert!(
                body.contains("on_exhausted: needs_human"),
                "{name} lost a key the rewrite does not own"
            );
            assert!(
                body.starts_with('#'),
                "{name} lost the comment that says it is yours now"
            );
            let parsed: serde_yaml_ng::Value = serde_yaml_ng::from_str(&body).unwrap();
            assert_eq!(parsed["budget"]["attempts"], 8);
        }
    }

    /// **A fresh guild is stamped**, so a later `armada guild upgrade` has a
    /// base to merge from. Before this, nothing anywhere said which template
    /// set a file came from, and the absence — not the merge — was what made
    /// upgrading impossible (`docs/reserved/006`).
    #[test]
    fn a_fresh_guild_records_which_templates_it_came_from() {
        let guild = tempfile::tempdir().unwrap();
        let wrote = write(guild.path(), &Answers::all_defaulted()).unwrap();
        assert!(wrote.contains(&crate::upstream::STAMP.to_string()));

        let body = std::fs::read_to_string(guild.path().join(crate::upstream::STAMP)).unwrap();
        let stamp = crate::upstream::read(&body).expect("the stamp parses");
        assert_eq!(stamp.templates, crate::upstream::digest());
        assert_eq!(stamp.version, crate::upstream::VERSION);
    }

    /// The skill and the persona are copied verbatim: nothing in the interview
    /// touches them, and `guild edit` is how they change.
    #[test]
    fn the_skill_and_the_persona_are_written_byte_for_byte() {
        let guild = tempfile::tempdir().unwrap();
        write(guild.path(), &Answers::all_defaulted()).unwrap();
        for starter in OTHERS {
            let written = std::fs::read_to_string(guild.path().join(starter.path)).unwrap();
            assert_eq!(written, starter.body, "{} was rewritten", starter.path);
        }
    }
}
