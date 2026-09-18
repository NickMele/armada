//! The rules `docs/concepts/studio.md` states about the sets, rather than the
//! sets themselves.
//!
//! Each one is a sentence on that page that nothing held: status colour on Run
//! and Job alone, a relation drawn by the Studio or a person's acceptance, a
//! promotion that ends inside Armada, and a node named a word the lexicon does
//! not ban. Split from [`super`] for size; the set comparisons are there.

use std::collections::BTreeSet;

use crate::Report;

use super::registry::Row;
use super::{as_wire, list, same_set, sources, Kinds, Side, LEXICON, MODEL, REGISTRY, STORE, WIRE};

/// The two kinds that draw in Job colours. `studio.md`: *Run and Job nodes are
/// the only nodes that take status colour*, because a run reads the same on a
/// Studio, on the Manifest surface and on a Job's run sheet.
const STATUS_COLOUR: &[&str] = &["run", "job"];

/// The two answers to *who draws a relation*, and there is no third: an agent
/// reorganising a person's work is what separates a drawing surface from a
/// record of decisions.
const BY_THE_STUDIO: &str = "the Studio";
const BY_ACCEPTANCE: &str = "a person accepts";

/// Who may take a rung of promotion. A `who` outside this is a route nothing
/// here can reason about.
const WHO: &[&str] = &[
    "person",
    "helm_on_ask",
    "scout_on_ask",
    "fleet",
    "dispatch_gate",
];

/// What a promotion may start from besides a node kind. **Neither is a legal
/// `to`**: capture and reading in both begin somewhere Armada does not own, and
/// nothing may end there. That is *nothing reaches outside Armada on a Studio's
/// behalf*, written as a set lookup.
const FROM_ONLY: &[&str] = &["any", "outside"];

/// A word the lexicon bans that a node kind carries anyway, and the term the
/// ban sits under.
///
/// **One, and it is scoped.** `run` is banned under **Job** — "Never run,
/// ticket" — which bans calling a *Job* a run. The Run node is a run: a
/// Manifest command, the same word `packages/tokens/src/status.css` uses for
/// one. Kept here rather than as a key in the registry, so widening it is an
/// edit to the gate and not to data.
const SANCTIONED: &[(&str, &str)] = &[("run", "Job")];

/// Status colour is Run's and Job's, and a kind that takes it declares no
/// state: its state is the run's or the Job's own.
pub fn status_colour(kinds: &Kinds, report: &mut Report) {
    let declared = kinds.flagged("status_colour", report);
    let want: BTreeSet<String> = STATUS_COLOUR.iter().map(|k| (*k).to_string()).collect();
    for kind in declared.difference(&want) {
        report.fail(format!(
            "{REGISTRY} — `{kind}` declares `status_colour`, and {} are the only kinds that take \
             it. A third would draw a Job's colours over something that is not a Job's",
            list(&want)
        ));
    }
    for kind in want.difference(&declared) {
        report.fail(format!(
            "{REGISTRY} — `{kind}` does not declare `status_colour`, and it is one of the two \
             kinds that take it. A run reads the same on a Studio as on a run sheet"
        ));
    }
    for row in &kinds.nodes {
        if !row.bools.get("status_colour").copied().unwrap_or(false) {
            continue;
        }
        let Some(states) = row.array("states", REGISTRY, report) else {
            continue;
        };
        if !states.is_empty() {
            report.fail(format!(
                "{REGISTRY}:{} — `{}` takes status colour and declares states of its own. Its \
                 state is read off the run or the Job, so a state written here would be right \
                 once",
                row.line,
                row.header()
            ));
        }
    }
}

/// Each kind's states are the ones `StudioNodeKind::states` answers, and each
/// is a state the registry declares.
pub fn states_per_kind(kinds: &Kinds, model: &str, report: &mut Report) {
    let spellings = sources::spelled(model, "StudioNodeState");
    let by_kind = sources::states_by_kind(model);
    if by_kind.is_empty() {
        report.fail(format!(
            "{MODEL} — `StudioNodeKind::states` was read as naming no kind at all, so every \
             kind's states were compared against nothing"
        ));
        return;
    }
    let kind_variants = sources::spelled(model, "StudioNodeKind");
    for row in &kinds.nodes {
        let Some(states) = row.array("states", REGISTRY, report) else {
            continue;
        };
        let ours: BTreeSet<String> = states.iter().cloned().collect();
        for state in &ours {
            if !kinds.states.contains(state) {
                report.fail(format!(
                    "{REGISTRY}:{} — `{}` names the state `{state}`, which this file has no \
                     `[states.{state}]` row for",
                    row.line,
                    row.header()
                ));
            }
        }
        let Some(variant) = kind_variants.get(&row.key) else {
            continue;
        };
        let Some(theirs) = by_kind.get(variant) else {
            report.fail(format!(
                "{MODEL} — `StudioNodeKind::{variant}` has no arm in `states`, so what `{}` may \
                 hold is compared against nothing",
                row.header()
            ));
            continue;
        };
        let theirs = as_wire(theirs, &spellings);
        if ours != theirs {
            report.fail(format!(
                "{REGISTRY}:{} — `{}` holds {}, and `StudioNodeKind::{variant}::states` answers \
                 {}. A state one side admits and the other refuses is a node that cannot be \
                 written or one that cannot be read",
                row.line,
                row.header(),
                list(&ours),
                list(&theirs)
            ));
        }
    }
}

/// A relation is drawn by the Studio or by a person's acceptance, and by
/// nothing else — held against the enum that carries the narrower set, the
/// store's own constraint, and what Bridge may ask for.
pub fn drawn_by(kinds: &Kinds, model: &str, store: &str, wire: &str, report: &mut Report) {
    let (mut studios, mut accepted) = (BTreeSet::new(), BTreeSet::new());
    for row in &kinds.edges {
        let Some(who) = row.string("drawn_by", REGISTRY, report) else {
            continue;
        };
        match who {
            BY_THE_STUDIO => {
                studios.insert(row.key.clone());
            }
            BY_ACCEPTANCE => {
                accepted.insert(row.key.clone());
            }
            _ => report.fail(format!(
                "{REGISTRY}:{} — `{}` is drawn by `{who}`, which is neither `{BY_THE_STUDIO}` \
                 nor `{BY_ACCEPTANCE}`. Only a person accepts a relation; Helm and a scout \
                 propose one",
                row.line,
                row.header()
            )),
        }
    }
    same_set(
        report,
        "relation a person accepts",
        &accepted,
        &super::wires(model, "StudioRelation"),
        Side {
            path: MODEL,
            absent_there: "no call can propose it",
            absent_here: "a relation can be proposed that the registry never sanctioned",
        },
    );
    same_set(
        report,
        "relation a person accepts",
        &accepted,
        &sources::literals(wire, "ProposeStudioEdge"),
        Side {
            path: WIRE,
            absent_there: "Bridge cannot propose it",
            absent_here: "Bridge may propose an edge kind that is the Studio's own to draw",
        },
    );
    same_set(
        report,
        "edge the Studio draws itself",
        &studios,
        &sources::accepted_on_sight(store),
        Side {
            path: STORE,
            absent_there: "a row may hold one still `proposed`, which is the Studio's own edge \
                           waiting on a person who was never asked",
            absent_here: "the `CHECK` names an edge kind the registry does not say the Studio \
                          draws",
        },
    );
}

/// The kinds a person puts on a Studio directly are the ones the enum answers
/// for and the ones the wire offers. Every other kind is made by the act that
/// earns it.
pub fn added_by_hand(kinds: &Kinds, model: &str, wire: &str, report: &mut Report) {
    let ours = kinds.flagged("added_by_hand", report);
    let spellings = sources::spelled(model, "StudioNodeKind");
    let variants: Vec<String> = sources::added_by_hand(model).into_iter().collect();
    same_set(
        report,
        "kind a person adds by hand",
        &ours,
        &as_wire(&variants, &spellings),
        Side {
            path: MODEL,
            absent_there: "Fleet refuses it from Bridge as `fleet.studio_node_not_a_persons`",
            absent_here: "a kind can be added by hand carrying a claim nothing stands behind",
        },
    );
    same_set(
        report,
        "kind a person adds by hand",
        &ours,
        &sources::tags(wire, "StudioNodeByHand"),
        Side {
            path: WIRE,
            absent_there: "the renderer cannot ask for it",
            absent_here: "the renderer may ask for a kind Fleet refuses, which is a capability \
                          wider than the act it carries",
        },
    );
}

/// Every rung ends at a node kind, starts at one or at a declared sentinel, and
/// names who acts from a closed set. The rung that makes a Job says the same
/// thing as `dispatches` on a node row, and the two are held equal.
pub fn promotions(kinds: &Kinds, report: &mut Report) {
    let nodes = kinds.node_keys();
    let mut dispatches_from = BTreeSet::new();
    for row in &kinds.promotions {
        let rung = row
            .string("rung", REGISTRY, report)
            .unwrap_or("")
            .to_string();
        ends(row, &nodes, &rung, report);
        starts(row, &nodes, &rung, report);
        if let Some(who) = row.array("who", REGISTRY, report) {
            for one in who {
                if !WHO.contains(&one.as_str()) {
                    report.fail(format!(
                        "{REGISTRY}:{} — the `{rung}` rung says `{one}` acts, which is none of \
                         the answers this file declares",
                        row.line
                    ));
                }
            }
        }
        let makes_a_job = row
            .arrays
            .get("to")
            .is_some_and(|to| to.iter().any(|kind| kind == "job"));
        if makes_a_job {
            if let Some(from) = row.arrays.get("from") {
                dispatches_from.extend(from.iter().filter(|k| nodes.contains(*k)).cloned());
            }
        }
    }
    let flagged = kinds.flagged("dispatches", report);
    if flagged != dispatches_from {
        report.fail(format!(
            "{REGISTRY} — {} declare `dispatches`, and the rung that makes a Job starts from {}. \
             The two say the same thing, so a kind in one and not the other is a Dispatch offered \
             where nothing is filed or withheld where something is",
            list(&flagged),
            list(&dispatches_from)
        ));
    }
}

/// Every `to` is a node kind, which is the whole of *no promotion writes
/// outside Armada*: a rung that did would have to end somewhere this file has
/// no row for.
fn ends(row: &Row, nodes: &BTreeSet<String>, rung: &str, report: &mut Report) {
    let Some(to) = row.array("to", REGISTRY, report) else {
        return;
    };
    if to.is_empty() {
        report.fail(format!(
            "{REGISTRY}:{} — the `{rung}` rung makes nothing. A rung that produces no node is \
             not a promotion",
            row.line
        ));
    }
    for kind in to {
        if !nodes.contains(kind) {
            report.fail(format!(
                "{REGISTRY}:{} — the `{rung}` rung ends at `{kind}`, which is no node kind this \
                 file declares. Nothing reaches outside Armada on a Studio's behalf, so every \
                 rung ends on the Studio",
                row.line
            ));
        }
    }
}

/// Every `from` is a node kind or one of the two sentinels.
fn starts(row: &Row, nodes: &BTreeSet<String>, rung: &str, report: &mut Report) {
    let Some(from) = row.array("from", REGISTRY, report) else {
        return;
    };
    for kind in from {
        if !nodes.contains(kind) && !FROM_ONLY.contains(&kind.as_str()) {
            report.fail(format!(
                "{REGISTRY}:{} — the `{rung}` rung starts from `{kind}`, which is neither a node \
                 kind this file declares nor one of the two this file allows a rung to start at",
                row.line
            ));
        }
    }
}

/// No node is named a word the lexicon says never to use, and no node is named
/// a word another node row already rejected.
pub fn lexicon(root: &std::path::Path, kinds: &Kinds, report: &mut Report) {
    let Some(text) = super::source(
        root,
        LEXICON,
        "the lexicon whose *Never* lists a node name may not be on",
        report,
    ) else {
        return;
    };
    let never = sources::never_words(&text);
    if never.is_empty() {
        report.fail(format!(
            "{LEXICON} — its lexicon was read as banning no word at all, so every node name was \
             compared against nothing"
        ));
        return;
    }
    let mut rejected: Vec<(String, String)> = Vec::new();
    for row in &kinds.nodes {
        if let Some(words) = row.array("not_called", REGISTRY, report) {
            rejected.extend(words.iter().map(|w| (w.to_lowercase(), row.key.clone())));
        }
    }
    for row in &kinds.nodes {
        let Some(name) = row.string("name", REGISTRY, report) else {
            continue;
        };
        let name = name.to_lowercase();
        // Sanctioned, and the const above says by whom and why. No finding:
        // a warning that never changes is one nobody reads.
        if SANCTIONED.iter().any(|(word, _)| *word == name) {
            continue;
        }
        if never.contains(&name) {
            report.fail(format!(
                "{REGISTRY}:{} — `{}` is named `{name}`, which {LEXICON}'s lexicon says never to \
                 use. Names avoid words Armada already uses",
                row.line,
                row.header()
            ));
        }
        if let Some((_, whose)) = rejected.iter().find(|(word, _)| *word == name) {
            report.fail(format!(
                "{REGISTRY}:{} — `{}` is named `{name}`, which `{whose}` rejects as a word \
                 Armada already uses. One of the two rows is wrong",
                row.line,
                row.header()
            ));
        }
    }
}
