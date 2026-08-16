//! `armada manifest prune` — reclaim disk, including disk that is not Armada's.
//!
//! **Every other reclaiming verb in Armada acts only on resources it created and
//! can prove it owns.** This one can reach further, and that is the entire
//! reason it is a separate verb rather than a flag on `clean`: a flag that could
//! delete somebody's database is a flag that eventually does.
//!
//! # The measurement this was written for
//!
//! A macOS storage warning, on a machine holding **171 local volumes and
//! 12.0 GB, 100% reclaimable — and not one of them carrying an Armada label.**
//! So the useful verb is one that can offer to remove things Armada did not
//! make; and the dangerous verb is the same verb, because *"reclaimable"* is
//! docker's word for "no container is currently using it", which is also true of
//! a database volume between runs.
//!
//! # The three rules, and none of them is a default
//!
//! 1. **A preview is mandatory.** Not `--dry-run`-by-request: there is no
//!    invocation that goes straight to removing. `armada fleet reap` is the
//!    established shape — rows toggle, enter confirms, esc touches nothing — and
//!    this reuses that selector rather than growing a second one.
//! 2. **Armada's own may open ticked. Nothing else may, ever.** Encoded in
//!    [`disk::default_ticks`] so it is one rule in one place rather than a habit
//!    two call sites share.
//! 3. **An unlabelled resource is never removed without a per-run confirmation
//!    from a person at a terminal.** Not a flag — a flag lives in a shell script
//!    or an agent's argv for ever. So `--json` and a pipe cannot remove one at
//!    all, whatever they pass.
//!
//! Rule 3 makes the verb less useful in automation. That is the correct trade:
//! the thing being traded away is an agent's ability to delete data it did not
//! create and cannot identify.
//!
//! # What `--json` and a pipe *can* do
//!
//! They can remove Armada's own, on `--yes`, and they can **list** everything.
//! Listing is the half that is always safe and is most of the value: the reader
//! asked what is using the disk, and the honest answer on the measured machine
//! is *"almost none of it is mine"*. Refusing to say so because Armada may not
//! delete it would answer a question nobody asked.

use armada_core::ctx::{Clock, Fetch, Run};
use armada_core::disk::{self, Confirmed, Ownership, PruneCandidate};
use armada_core::envelope::{Envelope, PruneData, PruneRow};
use armada_core::error::{ArmadaError, Status};
use armada_core::reap::PathStat;
use armada_manifest::{docker, fs};

use crate::app::App;
use crate::args::Common;
use crate::ask::{Ask, Choice};
use crate::verbs::Output;

/// What `prune` was asked to do beyond looking.
#[derive(Debug, Clone, Copy)]
pub struct Filters {
    /// Act without opening the tick list.
    ///
    /// **Consent for Armada's own and for nothing else** ([`Confirmed::ByAFlag`]).
    /// A flag is durable — it survives in a script, in an alias, in an agent's
    /// argv — and consent to delete unrecoverable data has to be given on the
    /// run that does it.
    pub yes: bool,
}

/// Run it.
pub fn run<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    common: Common,
    filters: Filters,
    ask: &mut dyn Ask,
    interactive: bool,
) -> Result<Output, ArmadaError> {
    // The daemon is an `environment` question, asked up front: `docker system
    // df` is not client-side, and discovering it at the end would mean doing the
    // whole survey first.
    app.docker_ready()?;

    let mut skipped: Vec<String> = Vec::new();
    let candidates = survey(app, &mut skipped)?;

    // **A preview is a run in which nothing is removed**, so `--dry-run` needs
    // no separate path: it is the ordinary run with the confirmation withheld.
    // `interactive` is `terminal.can_ask() && !json`, decided at the entrypoint
    // where the terminal is — nothing below it reads a stream's tty-ness.
    let confirmed = match (common.dry_run, filters.yes, interactive) {
        (true, _, _) => Confirmed::NotAtAll,
        (false, true, _) => Confirmed::ByAFlag,
        (false, false, true) => Confirmed::ByAPersonThisRun,
        // Nobody to ask and no flag: a preview, which is the safe default and
        // the one an agent gets unless it says otherwise.
        (false, false, false) => Confirmed::NotAtAll,
    };

    let ticks = match confirmed {
        // The tick list is the confirmation. `esc` touches nothing, and so does
        // a terminal that would not cooperate.
        Confirmed::ByAPersonThisRun => match offer(&candidates, ask) {
            Some(ticked) => ticked,
            None => return Ok(preview(candidates, skipped, "nothing was confirmed")),
        },
        // `--yes` takes what the defaults would have opened with — which is
        // Armada's own, idle, and nothing else.
        Confirmed::ByAFlag => disk::default_ticks(&candidates),
        Confirmed::NotAtAll => {
            return Ok(preview(candidates, skipped, why_preview(common, filters)))
        }
    };

    Ok(remove(app, candidates, ticks, confirmed, skipped))
}

/// Everything on this daemon that a prune could reach, and whose it is.
///
/// **Volumes only, and deliberately.** They are the leak: a named volume
/// outlives `down` and outlives its container by design, which is what 171 of
/// came to 12.0 GB. Images and build cache are `docker image prune` and
/// `docker builder prune`, which do exactly the right thing already and which
/// Armada has no ownership story for — a pulled `postgres:16` is shared with
/// everything else on the machine and was never Armada's to offer.
fn survey<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    skipped: &mut Vec<String>,
) -> Result<Vec<PruneCandidate>, ArmadaError> {
    // Sizes, by name. An enumeration that fails is `skipped` rather than fatal —
    // the same channel `clean` keeps for it, and for the same reason: a size
    // Armada could not read is a row it can still offer.
    let sizes = match app.docker_volume_sizes() {
        Ok(sizes) => sizes,
        Err(error) => {
            skipped.push(format!("volume sizes: {}", error.message));
            Vec::new()
        }
    };

    // Ownership, from the daemon. Both label namespaces, because a volume
    // stamped before the rename is still Armada's for one release.
    let ours = app.docker_list_labelled(docker::Kind::Volume)?;

    let mut candidates = Vec::new();
    for volume in &sizes {
        let mine = ours.iter().find(|owned| owned.reference == volume.name);
        let (owner, in_use) = match mine {
            // **A live workspace's volume is Armada's and is not idle.** The
            // reaper's rule decides which: the path label is stat-ed, and only
            // `ENOENT` means gone. Anything else — `EACCES`, a hung mount — is
            // "in use", because the failure of guessing wrong here is stopping
            // a colleague's stack mid-run.
            Some(owned) => (
                Ownership::Armada,
                fs::stat(&owned.workspace_path) != PathStat::Missing,
            ),
            None => (Ownership::Unlabelled, false),
        };
        candidates.push(PruneCandidate {
            kind: disk::DiskKind::Volumes,
            reference: volume.name.clone(),
            bytes: volume.size,
            owner,
            in_use,
        });
    }

    // **Armada's own first, then everything else, biggest first within each.**
    // The reader's own resources are the ones they have to judge one at a time,
    // and putting the largest at the top of that group is what makes a long list
    // worth reading at all.
    candidates.sort_by(|a, b| {
        (a.owner == Ownership::Unlabelled)
            .cmp(&(b.owner == Ownership::Unlabelled))
            .then(b.bytes.unwrap_or(0).cmp(&a.bytes.unwrap_or(0)))
            .then(a.reference.cmp(&b.reference))
    });
    Ok(candidates)
}

/// Put the list to a person. `None` when they said no, or when the terminal
/// would not cooperate.
///
/// **One selector, not a second one.** `crates/helm/src/ask/select.rs` is the
/// widget `config scan` and `fleet reap` already use; a prune with its own
/// arrow keys would be a second set to keep in agreement with this one.
fn offer(candidates: &[PruneCandidate], ask: &mut dyn Ask) -> Option<Vec<bool>> {
    let options: Vec<Choice> = candidates
        .iter()
        .map(|candidate| {
            Choice::new(
                &candidate.reference,
                &match (candidate.owner, candidate.in_use) {
                    (Ownership::Armada, false) => {
                        format!("{}, armada's, idle", disk::format_maybe(candidate.bytes))
                    }
                    (Ownership::Armada, true) => format!(
                        "{}, armada's, in use by a live workspace",
                        disk::format_maybe(candidate.bytes)
                    ),
                    // **Said plainly, every time.** This is the line standing
                    // between a tidy machine and somebody's database.
                    (Ownership::Unlabelled, _) => format!(
                        "{}, NOT armada's — armada did not create this and cannot say what is \
                         in it",
                        disk::format_maybe(candidate.bytes)
                    ),
                },
            )
        })
        .collect();
    ask.pick(
        "which of these should armada remove? removing a volume cannot be undone",
        &options,
        &disk::default_ticks(candidates),
    )
}

/// Why this run is only a preview, in the words that name the flag to change it.
fn why_preview(common: Common, filters: Filters) -> String {
    if common.dry_run {
        return "--dry-run: nothing was removed".to_string();
    }
    if common.json || !filters.yes {
        // **Short enough to survive the column**, because it is the only line
        // that tells a caller how to make the verb act.
        return "no terminal to ask at; `--yes` removes armada's own".to_string();
    }
    "nothing was confirmed".to_string()
}

/// A run that removed nothing, and says so on every row.
fn preview(
    candidates: Vec<PruneCandidate>,
    skipped: Vec<String>,
    why: impl Into<String>,
) -> Output {
    let why = why.into();
    let ticks = disk::default_ticks(&candidates);
    let results: Vec<PruneRow> = candidates
        .into_iter()
        .zip(ticks)
        .map(|(candidate, ticked)| PruneRow {
            status: Status::Skipped,
            // **Short, because the column truncates and the owner is the half
            // worth keeping.** An earlier version spelled the whole rule on
            // every row and an 80-column terminal cut it at *"not armada's, so
            // it is never remov…"* — losing the word the reader needs on the
            // row, and repeating sixty characters on each of 171 of them. The
            // rule is stated once, in `withheld`, and the row says whose it is.
            detail: Some(match (ticked, candidate.owner) {
                (true, _) => "would go".to_string(),
                (false, Ownership::Unlabelled) => "not armada's".to_string(),
                (false, Ownership::Armada) => "armada's, in use".to_string(),
            }),
            reference: candidate.reference,
            kind: candidate.kind,
            owner: candidate.owner,
            bytes: candidate.bytes,
        })
        .collect();
    let mut withheld = vec![why];
    // **Said once, in full, where the column cannot cut it.** A reader looking
    // at a screen of untouched rows has to know it is a rule and not an
    // oversight — and 171 copies of the sentence is not a better way to say it.
    let theirs = results
        .iter()
        .filter(|row| row.owner == Ownership::Unlabelled)
        .count();
    if theirs > 0 {
        withheld.push(match theirs {
            1 => "1 of these is not armada's; only a person can remove it".to_string(),
            n => format!("{n} of these are not armada's; only a person can remove them"),
        });
    }
    Output::Prune(Box::new(Envelope::ok(
        "prune",
        None,
        Status::Skipped,
        PruneData {
            results,
            freed: Some(0),
            withheld,
            skipped,
        },
    )))
}

/// Remove what was ticked **and permitted**, and report every row either way.
fn remove<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    candidates: Vec<PruneCandidate>,
    ticks: Vec<bool>,
    confirmed: Confirmed,
    skipped: Vec<String>,
) -> Output {
    let mut results = Vec::new();
    let mut withheld = Vec::new();
    let mut freed = Some(0u64);

    for (candidate, ticked) in candidates.into_iter().zip(ticks) {
        // **Two gates, and both must pass.** The tick is the person's intent;
        // [`disk::permitted`] is the rule that intent cannot override from the
        // wrong place. A `--json` run that somehow ticked an unlabelled volume
        // still does not remove it.
        let allowed = disk::permitted(&candidate, confirmed);
        if ticked && !allowed {
            withheld.push(format!(
                "{}: not armada's, and only a person at a terminal may remove one",
                candidate.reference
            ));
        }
        if !ticked || !allowed {
            results.push(PruneRow {
                status: Status::Skipped,
                detail: Some(match allowed {
                    true => "not selected".to_string(),
                    false => "not armada's".to_string(),
                }),
                reference: candidate.reference,
                kind: candidate.kind,
                owner: candidate.owner,
                bytes: candidate.bytes,
            });
            continue;
        }

        let outcome = app.docker_remove(
            docker::Kind::Volume,
            std::slice::from_ref(&candidate.reference),
        );
        let failure = outcome.into_iter().find_map(|(_, error)| error);
        let (status, detail) = match &failure {
            None => {
                // One unreadable size makes the total unknown and keeps it
                // unknown: a figure quietly missing a contributor reads as a
                // complete answer.
                freed = match (freed, candidate.bytes) {
                    (Some(sum), Some(bytes)) => Some(sum + bytes),
                    _ => None,
                };
                (Status::Clean, None)
            }
            Some(error) => (Status::Failed, Some(error.message.clone())),
        };
        results.push(PruneRow {
            status,
            detail,
            reference: candidate.reference,
            kind: candidate.kind,
            owner: candidate.owner,
            bytes: candidate.bytes,
        });
    }

    let failed = results.iter().any(|row| row.status == Status::Failed);
    let removed = results.iter().any(|row| row.status == Status::Clean);
    let status = match (failed, removed) {
        (true, true) => Status::Partial,
        (true, false) => Status::Failed,
        (false, _) => Status::Clean,
    };
    let data = PruneData {
        results,
        freed,
        withheld,
        skipped,
    };
    Output::Prune(Box::new(Envelope::ok("prune", None, status, data)))
}
