//! Bare `armada` — the front door
//! ([`docs/reserved/020-the-tui-decided.md`]'s menu decision).
//!
//! **What changed, and why it is not a smaller thing than it looks.**
//! [`PLAN.md`](../../../../docs/PLAN.md) §15.1 gave the bare word to Helm:
//! *"typing `armada` with no arguments enters it"*. `020` took it back and gave
//! it a menu of the modules. The argument that won is short: entering Helm is
//! off by default on a machine anyway (`helm.enter`), so a bare word that
//! *usually refuses* is a worse front door than one that lists what is there.
//! `PLAN.md` §15.1's reasoning is kept rather than deleted, because it is what
//! the menu decision was weighed against.
//!
//! **Five rows, each with a status word and one line of fact**, Helm first
//! because it is who you talk to.
//!
//! # Holding `ARCHITECTURE.md` §1.9, which is the real risk here
//!
//! `020` names it and does not soften it: *"the Bridge may **read** all four
//! modules; it must never become where they read **each other**. One screen
//! touching everything is exactly how that boundary erodes, and nothing in a
//! test will catch it — `cargo xtask boundaries` checks the crate graph, not a
//! renderer's habits."* This module is the second such screen, so the discipline
//! is written down rather than intended:
//!
//! 1. **One function per row, and each takes only its own module's input.**
//!    `helm_row` is handed a path, `fleet_row` a Fleet `Where`, `guild_row` a
//!    `Guild`. None of them can read another module's data because none of them
//!    is given any — which is a property of the signatures rather than of
//!    anybody's care.
//! 2. **No row's outcome gates another's.** `rows` computes all five
//!    unconditionally. The tempting shortcut — *"there is no workspace here, so
//!    skip the fleet row"* — would be Manifest deciding what Fleet reports, and
//!    it is exactly the erosion `020` warns about. A module that cannot answer
//!    says so on its own row.
//! 3. **No aggregate.** There is no headline word over the five, so there is no
//!    field anywhere that would have to be computed from two modules at once.
//!    `020` refuses an aggregate over several Jobs on the grounds that a word
//!    derived from the worst row describes none of them; the same argument
//!    applies here, and it happens to be the structural guarantee as well.
//!
//! Two of the three are asserted rather than intended:
//! `every_row_is_built_from_its_own_module_and_none_gates_another` holds (2)
//! against a machine with nothing on it and a `Run` that fails every call, and
//! `the_envelopes_status_describes_the_command_and_not_the_modules` holds (3).
//! Both are in this module's own tests.
//!
//! # No new status words
//!
//! Every word comes from [`Status`], which `docs/glossary.md` fixes. The
//! mapping is deliberately dull:
//!
//! | Row | `READY` | otherwise |
//! |---|---|---|
//! | Helm | `helm.enter` is on | `DOWN` — the switch is off |
//! | manifest | this directory resolves to a workspace | `DOWN` — nothing claims it |
//! | guild | there is a guild | `DOWN` — `armada init` makes one |
//!
//! `DOWN` is Manifest's own word for *not standing up*, and that is what an off
//! switch, an unclaimed directory and an absent guild each are. Fleet and inbox
//! are the two that move: `WAITING` when something is waiting on **you**,
//! `RUNNING` when work is in flight, `OK` when neither.
//!
//! [`docs/reserved/020-the-tui-decided.md`]: ../../../../docs/reserved/020-the-tui-decided.md

use armada_core::ctx::{Clock, Run};
use armada_core::envelope::{Envelope, MenuData, MenuRow};
use armada_core::error::{ArmadaError, Status};
use armada_guild::layout::Guild;
use std::path::Path;

use crate::verbs::Output;

/// The modules the front door lists, **in the order it lists them**.
///
/// **Helm first, because it is who you talk to.** The rest follow the order a
/// person meets them in: what is running, what it wants from you, where you are,
/// and what you brought with you.
///
/// **`mcp` is not here and neither is `doctor`.** This is a list of modules, and
/// those are a transport and a check — a front door that listed every top-level
/// verb would be `--help`, which already exists and is one keystroke away.
pub const MODULES: [&str; 5] = ["helm", "fleet", "inbox", "manifest", "guild"];

/// Bare `armada` — every module, with a word and a fact.
///
/// **Read-only, and every row degrades rather than failing.** A front door that
/// refused because one module could not answer would be the least useful screen
/// in the tool: the whole reason to open it is not knowing what state anything
/// is in.
pub fn ls<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &crate::verbs::fleet::Where,
    guild: &Guild,
) -> Result<Output, ArmadaError> {
    Ok(Output::Menu(Box::new(Envelope::ok(
        "menu",
        None,
        // **The command's status, not the fleet's** (PLAN.md §3.1). This lists
        // what is there and exits 0 whenever it could look; the words that
        // describe the modules are on the rows, which is the whole point of
        // there being no aggregate.
        Status::Ok,
        MenuData {
            results: rows(run, now, place, guild),
        },
    ))))
}

/// The five rows, **all of them, whatever any one of them says**.
///
/// See this module's header for why no row may gate another.
fn rows<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &crate::verbs::fleet::Where,
    guild: &Guild,
) -> Vec<MenuRow> {
    vec![
        helm_row(&place.armada_home),
        fleet_row(run, now, place),
        inbox_row(now, place),
        manifest_row(run, &place.cwd),
        guild_row(guild),
    ]
}

/// Helm — **can you talk to it from this machine.**
///
/// **The switch and nothing else.** Whether a conversation was ever started is a
/// fact about a file in `~/.armada/helm/`, and it answers a question nobody
/// standing at a front door is asking: the one thing that decides whether
/// `armada helm` works at all is `helm.enter`, and it is free to read.
fn helm_row(armada_home: &Path) -> MenuRow {
    let on = crate::verbs::helm::entering_allowed(armada_home);
    MenuRow {
        module: "helm".to_string(),
        status: match on {
            true => Status::Ready,
            false => Status::Down,
        },
        fact: match on {
            true => "resumes your conversation".to_string(),
            false => "off on this machine".to_string(),
        },
        // **The switch, not the verb it gates, when the switch is off.** A row
        // offering `armada helm` beside the word `DOWN` would advertise the one
        // command that refuses.
        verb: match on {
            true => "armada helm".to_string(),
            false => crate::verbs::helm::ENABLE.to_string(),
        },
    }
}

/// Fleet — **counts, never a word derived from the worst Job.**
///
/// `020` settles the shape: *"`4 jobs · 2 need you · 1 stalled`"*, because
/// counts cannot be wrong while an aggregate status over several Jobs describes
/// no Job in particular.
///
/// **`WAITING` outranks `RUNNING`**, and that is not an aggregate over the Jobs —
/// it is a fact about the module: something in it is waiting on *you*, which is
/// nameable, which is what separates `WAITING` from `QUEUED` in the glossary.
fn fleet_row<R: Run, C: Clock>(run: &R, now: &C, place: &crate::verbs::fleet::Where) -> MenuRow {
    let listing = crate::verbs::fleet::ls(run, now, place, false, false);
    let (status, fact) = match &listing {
        Ok(Output::FleetLs(envelope)) => {
            let data = &envelope.data;
            let running = data
                .results
                .iter()
                .filter(|row| row.state == armada_core::fleet::JobState::Running)
                .count();
            let stalled = data
                .results
                .iter()
                .filter(|row| {
                    matches!(
                        row.state,
                        armada_core::fleet::JobState::Stalled
                            | armada_core::fleet::JobState::Silent
                    )
                })
                .count();
            let mut facts = vec![crate::render::format::count(data.results.len(), "job")];
            // Omitted at zero rather than printed as `0 need you`, for the
            // reason every other surface omits it: the value of the line is that
            // "needs me" stays a signal (PLAN.md §15.4).
            if data.needs_you > 0 {
                facts.push(format!("{} need you", data.needs_you));
            }
            if stalled > 0 {
                facts.push(format!("{stalled} stalled"));
            }
            let status = if data.needs_you > 0 {
                Status::Waiting
            } else if running > 0 {
                Status::Running
            } else {
                Status::Ok
            };
            (status, facts.join(" · "))
        }
        // **A Fleet that cannot be read is `DOWN` on its own row**, and the rest
        // of the menu is unaffected. This is where rule (2) in the header is
        // paid for: the failure is one row's, not the screen's.
        _ => (Status::Down, "the Job index is unreadable".to_string()),
    };
    MenuRow {
        module: "fleet".to_string(),
        status,
        fact,
        verb: "armada fleet ls".to_string(),
    }
}

/// The inbox — **what the fleet is waiting on you for.**
fn inbox_row<C: Clock>(now: &C, place: &crate::verbs::fleet::Where) -> MenuRow {
    let listing = crate::verbs::fleet::inbox(now, place, None, false);
    let (status, fact) = match &listing {
        Ok(Output::Inbox(envelope)) => match envelope.data.open {
            0 => (Status::Ok, "nothing open".to_string()),
            open => (
                Status::Waiting,
                format!(
                    "{} waiting on you",
                    crate::render::format::count(open, "question")
                ),
            ),
        },
        _ => (Status::Down, "the inbox is unreadable".to_string()),
    };
    MenuRow {
        module: "inbox".to_string(),
        status,
        fact,
        verb: "armada fleet inbox".to_string(),
    }
}

/// Manifest — **whether this directory is a workspace at all.**
///
/// **`resolve` and never `status`.** The question here is *where am I*, and
/// `status` answers *what is up*, which costs docker probes on a screen a person
/// opens to orient themselves. A directory that claims nothing resolves to a
/// `bad_config` error naming what it searched; that is `DOWN` on this row rather
/// than a failure of the menu.
fn manifest_row<R: Run>(run: &R, cwd: &Path) -> MenuRow {
    let (status, fact, verb) = match armada_manifest::discovery::resolve(run, cwd) {
        Ok(workspace) => (
            Status::Ready,
            format!("{} — this workspace", workspace.config_label),
            "armada manifest status",
        ),
        // **The verb that claims a repo, not the one that reports on it.**
        // `armada manifest status` in a directory with no `armada.yml` refuses,
        // and offering it here would be the row naming its own dead end.
        Err(_) => (
            Status::Down,
            "no armada.yml here".to_string(),
            "armada manifest init",
        ),
    };
    MenuRow {
        module: "manifest".to_string(),
        status,
        fact,
        verb: verb.to_string(),
    }
}

/// The guild — **what you brought with you.**
fn guild_row(guild: &Guild) -> MenuRow {
    if !guild.exists() {
        return MenuRow {
            module: "guild".to_string(),
            status: Status::Down,
            fact: "no guild yet".to_string(),
            // **`armada init`, which is the machine verb that makes one** —
            // `armada guild ls` over an absent guild lists nothing and explains
            // nothing.
            verb: "armada init".to_string(),
        };
    }
    let inventory = crate::verbs::guild::inventory_of(guild);
    MenuRow {
        module: "guild".to_string(),
        status: Status::Ready,
        fact: match inventory.is_empty() {
            true => "nothing in it yet".to_string(),
            false => inventory.facts().join(" · "),
        },
        verb: "armada guild ls".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use armada_core::ctx::{RunOutput, RunRequest, SpawnError};

    /// **No module's answer gates another's** — rule (2) of this module's
    /// `ARCHITECTURE.md` §1.9 discipline, asserted rather than intended.
    ///
    /// **The machine here has nothing at all**: an empty `$HOME`, a directory
    /// that claims no workspace, no guild, no Jobs, and a `Run` that fails every
    /// call it is given. Every one of the five rows still appears, in order,
    /// with a word. That is what makes *"there is no workspace here, so skip the
    /// fleet row"* a failing test rather than a plausible optimisation somebody
    /// adds later — and it is the one erosion `020` warns about that
    /// `cargo xtask boundaries` cannot see, because it is a habit and not a
    /// crate edge.
    #[test]
    fn every_row_is_built_from_its_own_module_and_none_gates_another() {
        let (_home, place, guild) = a_bare_machine();
        let rows = rows(&NothingRuns, &FixedClock, &place, &guild);

        assert_eq!(
            rows.iter()
                .map(|row| row.module.as_str())
                .collect::<Vec<_>>(),
            MODULES,
            "a module dropped off the front door, or the order moved"
        );
        for row in &rows {
            assert!(!row.fact.is_empty(), "`{}` had no fact", row.module);
            assert!(
                row.verb.starts_with("armada "),
                "`{}`'s row offers no verb to type",
                row.module
            );
        }
    }

    /// **No aggregate**, which is rule (3) and the structural half of the same
    /// guarantee.
    ///
    /// A headline word over the five is the one field that would have to be
    /// computed from two modules at once. There is nowhere to put one — the
    /// payload has `results` and nothing else — and the envelope's own status is
    /// the *command's*, not the machine's: it is `OK` whenever the front door
    /// could look, even with every module reporting `DOWN`.
    #[test]
    fn the_envelopes_status_describes_the_command_and_not_the_modules() {
        let (_home, place, guild) = a_bare_machine();
        let Ok(Output::Menu(envelope)) = ls(&NothingRuns, &FixedClock, &place, &guild) else {
            panic!("the front door refused on a machine with nothing on it");
        };
        assert_eq!(envelope.status, Status::Ok);
        assert!(envelope.error.is_none());
        assert!(
            envelope
                .data
                .results
                .iter()
                .any(|row| row.status == Status::Down),
            "this machine has nothing on it and no row said so"
        );
    }

    /// Every word on every row is one `docs/glossary.md` already fixes.
    ///
    /// **A front door is the last place to widen the vocabulary**, because a
    /// word invented here is one every module's own verbs would then disagree
    /// with. `Status` is an enum, so this cannot fail by *spelling* — what it
    /// catches is a row reaching for a word outside the five the mapping uses.
    #[test]
    fn no_row_invents_a_status_word() {
        let (_home, place, guild) = a_bare_machine();
        for row in rows(&NothingRuns, &FixedClock, &place, &guild) {
            assert!(
                matches!(
                    row.status,
                    Status::Ready | Status::Down | Status::Ok | Status::Running | Status::Waiting
                ),
                "`{}` reached for `{}`",
                row.module,
                row.status
            );
        }
    }

    /// A machine with nothing on it, and a directory that claims nothing.
    fn a_bare_machine() -> (tempfile::TempDir, crate::verbs::fleet::Where, Guild) {
        let home = tempfile::tempdir().unwrap();
        let armada_home = home.path().join(".armada");
        let place = crate::verbs::fleet::Where {
            home: home.path().to_path_buf(),
            armada_home: armada_home.clone(),
            cwd: home.path().to_path_buf(),
            exe: std::path::PathBuf::from("armada"),
            boot_id: String::new(),
        };
        let guild = Guild::at(&armada_home);
        (home, place, guild)
    }

    /// **Every subprocess fails**, so no row can be answered by a tool being
    /// present. Manifest's discovery shells out to git and gets nothing; the
    /// row still appears.
    struct NothingRuns;
    impl Run for NothingRuns {
        fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
            Err(SpawnError {
                program: request.argv.first().cloned().unwrap_or_default(),
                kind: armada_core::ctx::SpawnErrorKind::NotFound,
                message: "nothing is installed on this machine".to_string(),
            })
        }
    }

    struct FixedClock;
    impl Clock for FixedClock {
        fn wall_rfc3339(&self) -> String {
            String::new()
        }
        fn wall_ms(&self) -> u64 {
            0
        }
        fn mono(&self) -> u64 {
            0
        }
        fn sleep_until(&self, _: u64) {}
    }
}
