//! The other direction: an operation the inventory names and nothing serves.
//!
//! The rule beside this one runs from the routes to the inventory, and that is
//! the direction it argues for. It is not enough. Ten queries carried
//! `agent_access = "Yes"` and no route at all, and nothing could see them: the
//! forward rule reads what is served, so what is not served is invisible to it.
//!
//! **The allowance names a reason per operation, never a bare list.** An
//! exemption with no sentence beside it is the same silent default `#696`
//! existed to remove, and the gate prints these so the next person deciding
//! reads the argument rather than the gap.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::Report;

use super::{operations, rows, INVENTORY, TABLE};

/// An operation the inventory names, that nothing serves, and why not.
///
/// **A reason per row.** It is refused once the operation gains a route, so
/// the list shrinks on its own rather than outliving the decisions in it.
const NOT_BUILT: &[(&str, &str)] = &[
    (
        "pause_job",
        "Retired 2026-09-03 rather than pending. `#395` deleted the name from the six \
         documents that counted it as an act; `docs/concepts/drone.md` carries the dated note \
         and `docs/contracts/system-architecture.md` sends a reader here. Redirect and Kill \
         are what a person does to a working Drone",
    ),
    (
        "deny_dispatch",
        "What it would add is the pattern learning, and nothing implements that. Clearing an \
         unwanted proposal off the Board is already possible — `killed` is reachable from \
         every non-terminal status, so `kill_job` then `forget_job` removes it — which is \
         narrower than a refusal fed back to the proposer and is what exists",
    ),
    (
        "restart_fleet",
        "It cannot serve the case it is named for: a dead Fleet cannot restart itself, and \
         that path is Bridge calling `launchctl kickstart -k` directly. The reachable case — \
         a Fleet alive and answering, asked for a clean restart — has no caller",
    ),
    (
        "resume_interrupted_job",
        "Two acts already do its work and say which applies. A Job interrupted by a Fleet \
         crash lands at `escalated`; `restart_step` respawns on the stopped step, \
         `redispatch_job` throws the run away, and `JobSummary::resumption` is what tells a \
         person which. A third act meaning `the right one of those` would be Fleet choosing \
         for them at the moment the record is least trustworthy",
    ),
    (
        "alert.raised",
        "Fleet keeps no Alert record. `list_alerts` derives its two buckets from the Jobs it \
         already holds, so every change to that answer arrives as the `job.state_changed` \
         that caused it, and a kind of its own would publish one fact twice",
    ),
    (
        "review.ready",
        "A Job reaching a human gate is a `job.state_changed` into `awaiting_review`, which \
         is what the Needs-you tab is already drawn from. A second kind carrying the same \
         transition is two events a client has to reconcile into one row",
    ),
    (
        "usage.threshold",
        "Nothing crosses a threshold. `settings.budget-quota-floor-for-interactive-use` \
         records that no quantity reaches Armada from a Drone's stream, and the two cost \
         ceilings refuse the next dispatch rather than firing — which is a `queued_reason` \
         on a `job.state_changed`",
    ),
    (
        "evidence.submitted",
        "It would put a submission's payload on the one drop-oldest channel every Job shares, \
         which is what that channel's bound exists to keep off it. What a client sees of a \
         submission is the step move or the gate that followed, and the bundle is \
         `get_evidence`",
    ),
];

/// Rule: every operation the inventory names is served, or says why not.
pub fn every_operation_the_inventory_names_is_served(root: &Path) -> Report {
    let mut report = Report::new("every operation the inventory names is served, or says why not");

    let Ok(inventory) = fs::read_to_string(root.join(INVENTORY)) else {
        report.fail(format!("{INVENTORY} — the operation inventory itself"));
        return report;
    };
    let Ok(table) = fs::read_to_string(root.join(TABLE)) else {
        report.fail(format!("{TABLE} — the route table itself"));
        return report;
    };

    check(&inventory, &table, &mut report);
    report
}

/// Every check the rule makes, over the two files as text.
pub(super) fn check(inventory: &str, table: &str, report: &mut Report) {
    let named = operations(inventory);
    let served: Vec<String> = rows(table)
        .into_iter()
        .map(|(operation, _, _)| operation)
        .collect();
    if named.is_empty() || served.is_empty() {
        // A rule that compares nothing passes on every repository, including a
        // broken one. This is the failure the forward rule already refuses on
        // an empty `SERVED`, arriving from the other side.
        report.fail(format!(
            "{INVENTORY} named {} operations and {TABLE} served {} — a comparison of nothing",
            named.len(),
            served.len()
        ));
        return;
    }
    let allowed: BTreeMap<&str, &str> = NOT_BUILT.iter().copied().collect();

    for (operation, kind) in &named {
        let routed = served.contains(operation);
        match (routed, allowed.get(operation.as_str())) {
            (false, None) => report.fail(format!(
                "{INVENTORY} names `{operation}` ({kind}) and {TABLE} serves it at no route. \
                 Route it, or say in `xtask`'s NOT_BUILT why it has none — a name with \
                 nothing behind it is a call that 404s, or an event nothing ever publishes"
            )),
            // **Refused once it is built**, so the allowance shrinks on its own
            // rather than outliving the decision in it.
            (true, Some(_)) => report.fail(format!(
                "`{operation}` is served and is still in `xtask`'s NOT_BUILT. Delete the row: \
                 an allowance for something that exists is an argument nobody will reread"
            )),
            _ => continue,
        }
    }

    // Printed whether or not anything failed, because the list is the decision.
    // A person reading `FAIL` on the next operation should see, in the same
    // output, which ones are deliberately unbuilt and why.
    for (operation, because) in NOT_BUILT {
        report.warn(format!(
            "{INVENTORY} names `{operation}` and nothing serves it, deliberately: {because}"
        ));
    }
}

#[cfg(test)]
mod tests;
