//! What reading an Epic in a second time does to the Studio: what it makes,
//! what it takes back, and what it leaves standing. `#1405`.
//!
//! **A function of the graph and the answer, so it is tested as one.** Nothing
//! here fetches, writes or mints; it is handed the Studio as it is and the
//! issues the forge printed, and it answers with the three lists a write needs.
//!
//! **Narrowing takes back what the read-in made, and nothing a person has
//! touched.** What happens to a node after a read-in is the person's work, and
//! an act of theirs is not something a press now may undo — so the question
//! here is not *is this issue still wanted* but *is this node still only the
//! read-in's*. `docs/concepts/studio.md`, *Promotion*, holds the rule.

use core_model::{
    EpicRead, EpicTake, StudioEdgeKind, StudioGraph, StudioNode, StudioNodeContent, StudioNodeId,
    StudioNodeKind, StudioPosition,
};

use super::{laid_out, ACROSS, DOWN, DOWN_A_COLUMN};

/// What a read-in of an Epic leaves the Studio holding.
pub(crate) struct WhatTheEpicTook {
    /// One Issue node per issue the answer takes that is not already here, and
    /// where to place each.
    pub(crate) made: Vec<(StudioNodeContent, StudioPosition)>,
    /// The nodes the read-in made and is taking back.
    pub(crate) taken_back: Vec<StudioNodeId>,
    /// What the Epic now says about itself.
    pub(crate) read_in: EpicRead,
}

/// Which issues to take, which nodes to take back, and what the Epic then says.
///
/// `at` is where the person is looking, and is used only where this Epic has no
/// block of issues yet: **one Epic has one block for its life**, so widening it
/// fills the gaps in that block rather than starting a second one.
pub(crate) fn what_it_took(
    graph: &StudioGraph,
    epic: &StudioNode,
    take: EpicTake,
    read: &adapters::MilestoneRead,
    at: StudioPosition,
) -> WhatTheEpicTook {
    let corner = already_read(epic).and_then(|read| read.laid_out_from);
    let mine: Vec<&StudioNode> = made_by_the_read_in(graph, epic.id());
    let from = corner.or_else(|| corner_of(&mine)).unwrap_or(at);
    let taken = read.taking(take);

    let mut kept = 0u64;
    let mut standing: Vec<&StudioNode> = Vec::new();
    let mut taken_back: Vec<StudioNodeId> = Vec::new();
    for node in &mine {
        let address = node.content().address().unwrap_or_default();
        let wanted = taken.issues.iter().any(|issue| issue.address == address);
        if wanted {
            standing.push(node);
        } else if touched(graph, node, corner) {
            kept += 1;
            standing.push(node);
        } else {
            taken_back.push(node.id().clone());
        }
    }

    // Every cell the block already holds, so a widening fills its gaps rather
    // than laying a second grid over the first.
    let mut occupied: Vec<i64> = standing
        .iter()
        .filter_map(|node| cell(node, from))
        .collect();
    let here: Vec<&str> = standing
        .iter()
        .filter_map(|node| node.content().address())
        .collect();
    let mut made: Vec<(StudioNodeContent, StudioPosition)> = Vec::new();
    let mut next = 0i64;
    for issue in &taken.issues {
        if here.contains(&issue.address.as_str()) {
            continue;
        }
        let Some(content) = issue.node() else {
            continue;
        };
        while occupied.contains(&next) {
            next += 1;
        }
        made.push((content, laid_out(from, next)));
        occupied.push(next);
        next += 1;
    }

    WhatTheEpicTook {
        read_in: EpicRead {
            issues: (standing.len() + made.len()) as u64,
            total: read.total,
            took: Some(take),
            left_out: taken.left_out,
            kept,
            laid_out_from: Some(from),
        },
        made,
        taken_back,
    }
}

/// What the Epic already says about itself, where it has been read in.
fn already_read(epic: &StudioNode) -> Option<EpicRead> {
    match epic.content() {
        StudioNodeContent::Epic { read_in, .. } => *read_in,
        _ => None,
    }
}

/// The Issue nodes this Epic produced — **the record that a read-in made
/// them.** A person never adds an Issue node by hand: they paste an address,
/// which makes a node with nothing above it. So an Issue hanging off this Epic
/// is one of its read-ins', and one that is not is somebody's own.
fn made_by_the_read_in<'a>(graph: &'a StudioGraph, epic: &StudioNodeId) -> Vec<&'a StudioNode> {
    graph
        .nodes
        .iter()
        .filter(|node| node.kind() == StudioNodeKind::Issue)
        .filter(|node| {
            graph
                .edges
                .iter()
                .any(|edge| produced_it(edge, epic, node.id()))
        })
        .collect()
}

fn produced_it(edge: &core_model::StudioEdge, from: &StudioNodeId, to: &StudioNodeId) -> bool {
    edge.kind() == StudioEdgeKind::Produced && edge.from() == from && edge.to() == to
}

/// Whether a person has worked on this node, so taking it back would undo an
/// act of theirs. Three marks, each written only by a person's act: an edge
/// beyond the one that made it, a line beside the address, and a position off
/// the block it was laid out in.
///
/// **The position is read only against a corner that was recorded.** An Epic
/// read in before `#1405` kept none, so where its issues sit says nothing —
/// and a board nothing could narrow would be the defect this replaced.
fn touched(graph: &StudioGraph, node: &StudioNode, corner: Option<StudioPosition>) -> bool {
    let beyond = graph
        .edges
        .iter()
        .filter(|edge| edge.from() == node.id() || edge.to() == node.id())
        .filter(|edge| edge.kind() != StudioEdgeKind::Produced || edge.from() == node.id())
        .count();
    let made_twice = graph
        .edges
        .iter()
        .filter(|edge| edge.kind() == StudioEdgeKind::Produced && edge.to() == node.id())
        .count()
        > 1;
    beyond > 0
        || made_twice
        || node.content().said().is_some()
        || corner.is_some_and(|corner| cell(node, corner).is_none())
}

/// Which cell of the block laid out from `corner` this node sits in, and `None`
/// where it sits between cells or outside the block — a node somebody moved.
fn cell(node: &StudioNode, corner: StudioPosition) -> Option<i64> {
    let (across, down) = (node.position().x - corner.x, node.position().y - corner.y);
    let fits = across % ACROSS == 0 && down % DOWN == 0 && across >= 0;
    let (column, row) = (across / ACROSS, down / DOWN);
    (fits && (0..DOWN_A_COLUMN).contains(&row)).then_some(column * DOWN_A_COLUMN + row)
}

/// The corner of a block whose own was never recorded: the topmost and
/// leftmost of what is in it. **A read-in fills its first cell first**, so
/// those two coordinates are the corner it laid out from — which is what lets
/// an Epic read in before `#1405` be narrowed rather than left as it landed.
fn corner_of(mine: &[&StudioNode]) -> Option<StudioPosition> {
    let x = mine.iter().map(|node| node.position().x).min()?;
    let y = mine.iter().map(|node| node.position().y).min()?;
    Some(StudioPosition { x, y })
}

#[cfg(test)]
mod tests;
