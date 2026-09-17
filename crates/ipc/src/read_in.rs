//! What a scout answers a read-in with, and how it is read back. `#1293`,
//! `docs/concepts/studio.md`, *Promotion*.
//!
//! **Here because this is where bytes enter the process.** A scout's answer is
//! text a model wrote, so it is decoded on the one seam that decodes, beside
//! every other message — and Fleet is handed values that are already typed.
//!
//! **Refused rather than salvaged, and bounded either way.** A block that does
//! not decode makes no node at all; one that does is capped, because what came
//! back landing on a Studio is unbounded otherwise.

use serde::{Deserialize, Serialize};

/// The most of each kind one read-in puts on a Studio.
pub const MOST_NOTES: usize = 24;
pub const MOST_CLUSTERS: usize = 6;
pub const MOST_CONTRADICTIONS: usize = 12;
pub const MOST_RELATIONS: usize = 24;
/// The most characters one node's text holds. A Note is what a source says in
/// a sentence or two, not the source again.
pub const MOST_CHARACTERS: usize = 2_000;

/// A handle the scout gave one node it is asking for, so a cluster and a
/// relation can name it. Meaningless outside the one answer it appears in.
pub type Handle = String;

/// One Note a read-in asks for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadInNote {
    pub id: Handle,
    pub said: String,
}

/// Notes the scout read as one thing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadInCluster {
    pub title: String,
    /// The handles of the Notes it groups.
    #[serde(default)]
    pub of: Vec<Handle>,
}

/// Two statements that disagree: what the source says, and what it disagrees
/// with. **Both are text**, quoted from each side, because one side is outside
/// the repository and has no node to point at.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadInContradiction {
    pub id: Handle,
    pub first: String,
    pub second: String,
}

/// A relation between two nodes this read-in made. **Proposed, never drawn**:
/// only a person accepts one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadInRelation {
    pub from: Handle,
    pub relation: String,
    pub to: Handle,
}

/// Everything one read-in asks the Studio for.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadIn {
    #[serde(default)]
    pub notes: Vec<ReadInNote>,
    #[serde(default)]
    pub clusters: Vec<ReadInCluster>,
    #[serde(default)]
    pub contradictions: Vec<ReadInContradiction>,
    #[serde(default)]
    pub relations: Vec<ReadInRelation>,
}

impl ReadIn {
    /// Whether it asks for nothing at all.
    pub fn empty(&self) -> bool {
        self.notes.is_empty() && self.clusters.is_empty() && self.contradictions.is_empty()
    }

    /// Every list cut to its bound and every text to [`MOST_CHARACTERS`], with
    /// a blank text dropped: a node with nothing on it is a node nobody reads.
    fn bounded(mut self) -> ReadIn {
        let cut = |text: &str| {
            text.trim()
                .chars()
                .take(MOST_CHARACTERS)
                .collect::<String>()
        };
        self.notes.truncate(MOST_NOTES);
        self.clusters.truncate(MOST_CLUSTERS);
        self.contradictions.truncate(MOST_CONTRADICTIONS);
        self.relations.truncate(MOST_RELATIONS);
        for note in &mut self.notes {
            note.said = cut(&note.said);
        }
        for cluster in &mut self.clusters {
            cluster.title = cut(&cluster.title);
        }
        for one in &mut self.contradictions {
            one.first = cut(&one.first);
            one.second = cut(&one.second);
        }
        self.notes.retain(|note| !note.said.is_empty());
        self.clusters.retain(|one| !one.title.is_empty());
        self.contradictions
            .retain(|one| !one.first.is_empty() && !one.second.is_empty());
        self
    }
}

/// What a scout asked for, read out of the answer it ended its turn with.
///
/// **The last fenced block, not the first.** A scout may quote the shape it was
/// asked for while explaining itself, and what it meant is what it finished
/// with.
pub fn what_a_scout_read_in(answered: &str) -> Result<ReadIn, crate::Undecodable> {
    let block = fenced(answered).unwrap_or(answered.trim());
    crate::decode::<ReadIn>("what a scout read in", block.as_bytes()).map(ReadIn::bounded)
}

/// The contents of the last fenced block in `text`, `None` where there is none.
fn fenced(text: &str) -> Option<&str> {
    let close = text.rfind("```")?;
    let open = text[..close].rfind("```")?;
    let body = &text[open + 3..close];
    // The fence's own language tag sits on the opening line and is not JSON.
    Some(match body.split_once('\n') {
        Some((head, rest)) if !head.contains('{') => rest,
        _ => body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scout writes prose and ends with the block, which is the shape the
    /// brief asks for — `docs/contracts/agent-prompt.md`, section 5c.
    #[test]
    fn the_last_fenced_block_is_what_a_scout_asked_for() {
        let answered = "Here is what I found.\n\n```json\n{\"notes\":[{\"id\":\"n1\",\
                        \"said\":\"The page dates the decision to June\"}]}\n```\n";
        let read = what_a_scout_read_in(answered).expect("the block decodes");
        assert_eq!(read.notes.len(), 1);
        assert_eq!(read.notes[0].id, "n1");
    }

    /// **An answer that is not the shape makes no node**, rather than one node
    /// holding whatever the model wrote: a Studio is read by agents, and a Note
    /// carrying an apology is a record of nothing.
    #[test]
    fn prose_with_no_block_is_refused_rather_than_kept_as_a_note() {
        assert!(what_a_scout_read_in("I could not read the page.").is_err());
    }

    /// Bounded on the way in, because what came back lands on a Studio that is
    /// kept until a person deletes it.
    #[test]
    fn what_came_back_is_cut_to_the_bounds_and_blank_text_makes_no_node() {
        let notes: Vec<String> = (0..MOST_NOTES + 5)
            .map(|n| format!("{{\"id\":\"n{n}\",\"said\":\"{}\"}}", "x".repeat(3_000)))
            .collect();
        let answered = format!(
            "```json\n{{\"notes\":[{},{{\"id\":\"b\",\"said\":\" \"}}]}}\n```",
            notes.join(",")
        );
        let read = what_a_scout_read_in(&answered).expect("it decodes");
        assert_eq!(read.notes.len(), MOST_NOTES);
        assert!(read
            .notes
            .iter()
            .all(|note| note.said.chars().count() == MOST_CHARACTERS));
    }
}
