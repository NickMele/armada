//! What a form declares whole — a Check, a Command, a port, `evidence:` — and
//! the map each is written as. Apart from `edits.rs` for its length.

use super::edits::{number, text, texts, Node};

/// A Check a form declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewCheck {
    pub run: String,
    pub requires: Vec<String>,
    pub when: Vec<String>,
    pub narrow: Option<NewNarrowing>,
}

/// `checks.<name>.narrow`, whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewNarrowing {
    pub run: String,
    pub each: String,
    pub from: Vec<String>,
    pub under: Option<String>,
    pub except: Vec<String>,
}

/// A Command a form declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewCommand {
    pub run: Option<String>,
    pub destructive: bool,
    pub serve: Option<String>,
    pub ready: Option<String>,
    pub links: Vec<NewLink>,
}

/// One address a server offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewLink {
    pub url: String,
    pub name: Option<String>,
}

/// A port a form declares. Both absent is `{}`, which is a port Armada places.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewPort {
    pub container: Option<u32>,
    pub env: Option<String>,
}

/// `evidence:`, whole. `serve` and `ready` go together or not at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewEvidence {
    pub serve: Option<String>,
    pub ready: Option<String>,
    pub run: String,
    pub frames: String,
    pub never: Vec<String>,
}

pub(super) fn links_node(links: &[NewLink]) -> Node {
    Node::List(
        links
            .iter()
            .map(|link| {
                let mut entry = vec![("url".to_string(), text(&link.url))];
                if let Some(name) = &link.name {
                    entry.push(("name".to_string(), text(name)));
                }
                Node::Map(entry)
            })
            .collect(),
    )
}

/// Builds a map in the order a Manifest's own key lists spell it, leaving out
/// what is absent.
struct Entries(Vec<(String, Node)>);

impl Entries {
    fn new() -> Entries {
        Entries(Vec::new())
    }

    fn with(mut self, key: &str, node: Option<Node>) -> Entries {
        if let Some(node) = node {
            self.0.push((key.to_string(), node));
        }
        self
    }

    fn list(self, key: &str, items: &[String]) -> Entries {
        let node = (!items.is_empty()).then(|| texts(items));
        self.with(key, node)
    }

    fn done(self) -> Node {
        Node::Map(self.0)
    }
}

impl NewCheck {
    pub(super) fn node(&self) -> Node {
        Entries::new()
            .with("run", Some(text(&self.run)))
            .list("requires", &self.requires)
            .list("when", &self.when)
            .with("narrow", self.narrow.as_ref().map(NewNarrowing::node))
            .done()
    }
}

impl NewNarrowing {
    pub(super) fn node(&self) -> Node {
        Entries::new()
            .with("run", Some(text(&self.run)))
            .with("each", Some(text(&self.each)))
            .list("from", &self.from)
            .with("under", self.under.as_deref().map(text))
            .list("except", &self.except)
            .done()
    }
}

impl NewCommand {
    pub(super) fn node(&self) -> Node {
        Entries::new()
            .with("run", self.run.as_deref().map(text))
            .with("destructive", self.destructive.then_some(Node::Flag(true)))
            .with("serve", self.serve.as_deref().map(text))
            .with("ready", self.ready.as_deref().map(text))
            .with(
                "links",
                (!self.links.is_empty()).then(|| links_node(&self.links)),
            )
            .done()
    }
}

impl NewPort {
    pub(super) fn node(&self) -> Node {
        Entries::new()
            .with("container", self.container.map(number))
            .with("env", self.env.as_deref().map(text))
            .done()
    }
}

impl NewEvidence {
    pub(super) fn node(&self) -> Node {
        Entries::new()
            .with("serve", self.serve.as_deref().map(text))
            .with("ready", self.ready.as_deref().map(text))
            .with("run", Some(text(&self.run)))
            .with("frames", Some(text(&self.frames)))
            .list("never", &self.never)
            .done()
    }
}
