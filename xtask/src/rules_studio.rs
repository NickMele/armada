//! A Studio's kinds are one set across the registry, the enums, the store and
//! the wire.
//!
//! They were spelled in three places nothing compared — `core-model`'s enums,
//! the store's `CHECK` constraints and the TypeScript mirror — so a kind added
//! to one and not the others merged green and failed the first time a person
//! met it. `crates/core-model/domain/studio-kinds.toml` is the authority, this
//! rule is what keeps it from being a fourth copy, and `#1313` is why.
//!
//! **The registry key is the wire value** and the comparison runs both ways,
//! for [`crate::rules_enums`]'s reasons. Beyond the sets it holds the four
//! rules `docs/concepts/studio.md` states and nothing checked: status colour on
//! Run and Job alone, a relation drawn by the Studio or a person's acceptance,
//! no promotion ending outside Armada, and no node named a word the lexicon
//! bans. **No `toml` crate and no `syn`** — the gate keeps no dependencies.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::Report;

pub mod holds;
pub mod registry;
pub mod sources;

const REGISTRY: &str = "crates/core-model/domain/studio-kinds.toml";
const MODEL: &str = "crates/core-model/src/studio.rs";
const STORE: &str = "crates/store/src/studio.rs";
const WIRE: &str = "packages/protocol/src/studio.ts";
const LEXICON: &str = "docs/contracts/design-system.md";

/// The registry, read once, as every check here needs it.
pub struct Kinds {
    pub nodes: Vec<registry::Row>,
    pub states: BTreeSet<String>,
    pub edges: Vec<registry::Row>,
    pub forge_states: Vec<registry::Row>,
    pub epic_take: BTreeSet<String>,
    pub promotions: Vec<registry::Row>,
}

impl Kinds {
    pub fn node_keys(&self) -> BTreeSet<String> {
        self.nodes.iter().map(|row| row.key.clone()).collect()
    }

    /// Every node kind whose `field` is true, naming any row that declares no
    /// answer at all.
    pub fn flagged(&self, field: &str, report: &mut Report) -> BTreeSet<String> {
        self.nodes
            .iter()
            .filter(|row| row.bool(field, REGISTRY, report).unwrap_or(false))
            .map(|row| row.key.clone())
            .collect()
    }
}

/// Rule: a Studio's kinds are one set across the registry, the enums, the store
/// and the wire.
pub fn the_kinds_are_one_set_everywhere(root: &Path) -> Report {
    let mut report = Report::new(
        "a Studio's kinds are one set across the registry, the enums, the store and the wire",
    );

    let Some(kinds) = read(root, &mut report) else {
        return report;
    };
    let Some(model) = source(
        root,
        MODEL,
        "the enums a Studio's sets reach code as",
        &mut report,
    ) else {
        return report;
    };
    let Some(store) = source(
        root,
        STORE,
        "the migrations whose `CHECK` sets are what a row may hold",
        &mut report,
    ) else {
        return report;
    };
    let Some(wire) = source(
        root,
        WIRE,
        "the mirror the whiteboard draws from",
        &mut report,
    ) else {
        return report;
    };

    sets(&kinds, &model, &store, &wire, &mut report);
    holds::status_colour(&kinds, &mut report);
    holds::states_per_kind(&kinds, &model, &mut report);
    holds::drawn_by(&kinds, &model, &store, &wire, &mut report);
    holds::added_by_hand(&kinds, &model, &wire, &mut report);
    holds::promotions(&kinds, &mut report);
    holds::lexicon(root, &kinds, &mut report);

    report
}

/// Every set, against every other place it is spelled.
fn sets(kinds: &Kinds, model: &str, store: &str, wire: &str, report: &mut Report) {
    let nodes = kinds.node_keys();
    same_set(
        report,
        "node kind",
        &nodes,
        &wires(model, "StudioNodeKind"),
        Side {
            path: MODEL,
            absent_there: "nothing on the wire can carry it and no node can be made of it",
            absent_here: "a kind reaches every surface that the registry never sanctioned",
        },
    );
    same_set(
        report,
        "node kind",
        &nodes,
        &sources::check_set(store, "studio_nodes", "kind"),
        Side {
            path: STORE,
            absent_there: "every write of one is refused by the `CHECK`, at runtime. Widening a \
                           `CHECK` is a new migration, never an edit to one that has run",
            absent_here: "a row may hold a kind nothing else knows, and it reads back as `None`",
        },
    );
    same_set(
        report,
        "node kind",
        &nodes,
        &sources::tags(wire, "StudioNodeContent"),
        Side {
            path: WIRE,
            absent_there: "the whiteboard meets a node whose content it cannot type",
            absent_here: "a client may send a kind Fleet has no variant for",
        },
    );

    same_set(
        report,
        "node state",
        &kinds.states,
        &wires(model, "StudioNodeState"),
        Side {
            path: MODEL,
            absent_there: "a row holding it reads back as unreadable",
            absent_here: "a state is written that the registry never sanctioned",
        },
    );
    same_set(
        report,
        "node state",
        &kinds.states,
        &sources::check_set(store, "studio_nodes", "state"),
        Side {
            path: STORE,
            absent_there: "every write of one is refused by the `CHECK`, at runtime",
            absent_here: "a row may hold a state no node kind declares",
        },
    );

    let edges: BTreeSet<String> = kinds.edges.iter().map(|row| row.key.clone()).collect();
    same_set(
        report,
        "edge kind",
        &edges,
        &wires(model, "StudioEdgeKind"),
        Side {
            path: MODEL,
            absent_there: "no edge of it can be drawn",
            absent_here: "an edge kind exists that the registry never sanctioned",
        },
    );
    same_set(
        report,
        "edge kind",
        &edges,
        &sources::check_set(store, "studio_edges", "kind"),
        Side {
            path: STORE,
            absent_there: "every write of one is refused by the `CHECK`, at runtime",
            absent_here: "a row may hold an edge kind nothing else knows",
        },
    );

    let forge: BTreeSet<String> = kinds.forge_states.iter().map(|r| r.key.clone()).collect();
    same_set(
        report,
        "forge state",
        &forge,
        &wires(model, "ForgeState"),
        Side {
            path: MODEL,
            absent_there: "an adapter has nothing to read a forge's word back as",
            absent_here: "a state reaches a node that the registry never sanctioned",
        },
    );
    same_set(
        report,
        "forge state",
        &forge,
        &sources::literals(wire, "ForgeState"),
        Side {
            path: WIRE,
            absent_there: "an Issue arrives in a state the whiteboard cannot draw",
            absent_here: "the mirror offers a state Fleet never sends",
        },
    );

    same_set(
        report,
        "Epic take",
        &kinds.epic_take,
        &wires(model, "EpicTake"),
        Side {
            path: MODEL,
            absent_there: "a read-in cannot be asked for it",
            absent_here: "an answer exists that the registry never sanctioned",
        },
    );
    same_set(
        report,
        "Epic take",
        &kinds.epic_take,
        &sources::literals(wire, "EpicTake"),
        Side {
            path: WIRE,
            absent_there: "Bridge cannot ask for it",
            absent_here: "Bridge may ask for an answer Fleet has no variant for",
        },
    );

    for row in &kinds.forge_states {
        let Some(on) = row.array("on", REGISTRY, report) else {
            continue;
        };
        for kind in on {
            if !nodes.contains(kind) {
                report.fail(format!(
                    "{REGISTRY}:{} — `{}` says `{kind}` may carry it, and `{kind}` is no node \
                     kind this file declares",
                    row.line,
                    row.header()
                ));
            }
        }
    }
}

/// One other spelling of a set, and what a gap on each side costs.
pub struct Side {
    pub path: &'static str,
    pub absent_there: &'static str,
    pub absent_here: &'static str,
}

/// Two spellings of one set, compared both ways.
///
/// **An empty other side is a failure, not agreement.** A reader that stops
/// matching the shape it reads compares nothing and reports ok, which is what
/// [`crate::rules_enums`] found by breaking a spelling on purpose.
pub fn same_set(
    report: &mut Report,
    what: &str,
    ours: &BTreeSet<String>,
    theirs: &BTreeSet<String>,
    side: Side,
) {
    let Side {
        path,
        absent_there,
        absent_here,
    } = side;
    if theirs.is_empty() {
        report.fail(format!(
            "{path} — not one {what} was read out of it. Either it spells none, which has never \
             been true here, or it no longer has the shape this rule reads — either way the set \
             was compared against nothing, which reads as agreement"
        ));
        return;
    }
    for value in ours.difference(theirs) {
        report.fail(format!(
            "`{value}` is a {what} {REGISTRY} declares and {path} does not spell, so \
             {absent_there}"
        ));
    }
    for value in theirs.difference(ours) {
        report.fail(format!(
            "{path} spells `{value}` as a {what}, and {REGISTRY} has no row for it, so \
             {absent_here}. The registry is the authority on the set"
        ));
    }
}

/// One enum's wire spellings, as a set.
fn wires(model: &str, name: &str) -> BTreeSet<String> {
    sources::spelled(model, name).into_keys().collect()
}

/// The registry, sorted into the tables the checks read, or a finding.
fn read(root: &Path, report: &mut Report) -> Option<Kinds> {
    let text = source(
        root,
        REGISTRY,
        "the authority on every set a Studio's graph is made of",
        report,
    )?;
    let mut rows = registry::rows(&text, REGISTRY, report);
    let mut take = |table: &str| -> Vec<registry::Row> {
        let (mine, rest) = rows.drain(..).partition(|row| row.table == table);
        rows = rest;
        mine
    };
    let kinds = Kinds {
        nodes: take("nodes"),
        states: take("states").iter().map(|r| r.key.clone()).collect(),
        edges: take("edges"),
        forge_states: take("forge_states"),
        epic_take: take("epic_take").iter().map(|r| r.key.clone()).collect(),
        promotions: take("promotions"),
    };
    for row in &rows {
        report.fail(format!(
            "{REGISTRY}:{} — `{}` is under a table no check here reads, so everything on it is \
             compared against nothing",
            row.line,
            row.header()
        ));
    }
    for (table, empty) in [
        ("nodes", kinds.nodes.is_empty()),
        ("states", kinds.states.is_empty()),
        ("edges", kinds.edges.is_empty()),
        ("forge_states", kinds.forge_states.is_empty()),
        ("epic_take", kinds.epic_take.is_empty()),
        ("promotions", kinds.promotions.is_empty()),
    ] {
        if empty {
            report.fail(format!(
                "{REGISTRY} — no `{table}` row at all. Either the table is gone or it no longer \
                 has the shape this rule reads, and either way everything keyed on it was \
                 compared against nothing"
            ));
            return None;
        }
    }
    Some(kinds)
}

/// A file, or a finding naming what it is that could not be read.
fn source(root: &Path, path: &'static str, what: &str, report: &mut Report) -> Option<String> {
    match fs::read_to_string(root.join(path)) {
        Ok(text) => Some(text),
        Err(_) => {
            report.fail(format!("{path} — {what}"));
            None
        }
    }
}

/// A set as a finding should say it. `nothing` rather than `[]`: an empty
/// bracket in a sentence reads as a formatting slip.
pub fn list(values: &BTreeSet<String>) -> String {
    match values.is_empty() {
        true => String::from("nothing"),
        false => values
            .iter()
            .map(|v| format!("`{v}`"))
            .collect::<Vec<String>>()
            .join(", "),
    }
}

/// Variant names as wire spellings, so a comparison runs in the registry's own
/// terms. A variant with no spelling is carried through as itself, and the set
/// comparison is what names it.
pub fn as_wire(variants: &[String], spellings: &BTreeMap<String, String>) -> BTreeSet<String> {
    let by_variant: BTreeMap<&str, &str> = spellings
        .iter()
        .map(|(wire, variant)| (variant.as_str(), wire.as_str()))
        .collect();
    variants
        .iter()
        .map(|variant| match by_variant.get(variant.as_str()) {
            Some(wire) => (*wire).to_string(),
            None => variant.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests;
