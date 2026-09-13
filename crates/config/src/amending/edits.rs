//! What a form can say, and what each one does to the file.
//!
//! **One variant per thing a form edits, and nothing that names a key by
//! string.** A dotted path from a caller would let a form reach a key nothing
//! here decided it may, so the vocabulary is closed and every path is spelled
//! below.
//!
//! **An empty list is an absent key.** `yaml::list` refuses `requires: []`
//! outright, so clearing a list removes the key rather than writing one that
//! would not load.

use core_model::{AutoMerge, ReviewGate};
use serde_yaml_ng::{Mapping, Number, Value};

use super::{NotAmended, Unplaceable};

/// One edit a form makes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    Check {
        name: String,
        edit: CheckEdit,
    },
    Command {
        name: String,
        edit: CommandEdit,
    },
    Port {
        name: String,
        edit: PortEdit,
    },
    /// `auto_merge`. `None` removes the key, which means `never`.
    AutoMerge(Option<AutoMerge>),
    /// `review_gate`. `None` removes the key, which means `human_always`.
    ReviewGate(Option<ReviewGate>),
    /// `drone.cost_cap_micros_per_job`. `None` defers to what Fleet runs with.
    CostCapMicrosPerJob(Option<u32>),
    /// `drone.turn_cap_per_job`. `None` defers to what Fleet runs with.
    TurnCapPerJob(Option<u32>),
}

/// An edit to one Check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckEdit {
    /// Declared last, which is where it starts in the gate's order.
    Add(NewCheck),
    Remove,
    Run(String),
    Requires(Vec<String>),
    When(Vec<String>),
    Narrow(Option<NewNarrowing>),
}

/// An edit to one Command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandEdit {
    Add(NewCommand),
    Remove,
    Run(Option<String>),
    /// **Set here or nowhere** — Scan can only propose it and Verify cannot
    /// check it. `false` removes a written `true`, because absent already means
    /// `false`, and leaves a written `false` alone.
    Destructive(bool),
    Serve(Option<String>),
    Ready(Option<String>),
    Links(Vec<NewLink>),
}

/// An edit to one port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortEdit {
    Add(NewPort),
    Remove,
    Container(Option<u32>),
    Env(Option<String>),
}

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

/// A value to write, in the shapes a Manifest has.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Node {
    Text(String),
    Flag(bool),
    Number(Number),
    List(Vec<Node>),
    Map(Vec<(String, Node)>),
}

/// One set or removal at a path of keys from the top of the document.
#[derive(Debug, Clone)]
pub(crate) struct Op {
    pub(crate) path: Vec<String>,
    /// `None` removes the key.
    pub(crate) to: Option<Node>,
    /// Whether a removal takes the comment block written directly above the
    /// key. **Only an entry a form removes** — a Check, a Command, a port, or
    /// the section its last one leaves empty — never a field or a dial.
    pub(crate) attached: bool,
}

impl Op {
    fn set(path: &[&str], to: Node) -> Op {
        Op {
            path: path.iter().map(|key| key.to_string()).collect(),
            to: Some(to),
            attached: false,
        }
    }

    fn remove(path: &[&str]) -> Op {
        Op {
            path: path.iter().map(|key| key.to_string()).collect(),
            to: None,
            attached: false,
        }
    }

    /// An entry out, with the comment block directly above it.
    fn remove_entry(path: &[&str]) -> Op {
        Op {
            attached: true,
            ..Op::remove(path)
        }
    }

    /// The dotted key a refusal names.
    pub(crate) fn key(&self) -> String {
        self.path.join(".")
    }
}

impl Edit {
    /// The key a refusal about the whole edit names.
    fn key(&self) -> String {
        match self {
            Edit::Check { name, .. } => format!("checks.{name}"),
            Edit::Command { name, .. } => format!("commands.{name}"),
            Edit::Port { name, .. } => format!("ports.{name}"),
            Edit::AutoMerge(_) => "auto_merge".to_string(),
            Edit::ReviewGate(_) => "review_gate".to_string(),
            Edit::CostCapMicrosPerJob(_) => "drone.cost_cap_micros_per_job".to_string(),
            Edit::TurnCapPerJob(_) => "drone.turn_cap_per_job".to_string(),
        }
    }

    pub(crate) fn unplaceable(&self, why: Unplaceable) -> NotAmended {
        NotAmended::Unplaceable {
            key: self.key(),
            why,
        }
    }

    /// What this edit does to a document that reads as `doc`.
    pub(crate) fn ops(&self, doc: &Value) -> Result<Vec<Op>, NotAmended> {
        match self {
            Edit::Check { name, edit } => check(doc, name, edit),
            Edit::Command { name, edit } => command(doc, name, edit),
            Edit::Port { name, edit } => port(doc, name, edit),
            Edit::AutoMerge(word) => Ok(vec![policy(
                "auto_merge",
                word.as_ref().map(AutoMerge::as_written),
            )]),
            Edit::ReviewGate(word) => Ok(vec![policy(
                "review_gate",
                word.as_ref().map(ReviewGate::as_written),
            )]),
            Edit::CostCapMicrosPerJob(cap) => Ok(vec![dial(doc, "cost_cap_micros_per_job", *cap)]),
            Edit::TurnCapPerJob(cap) => Ok(vec![dial(doc, "turn_cap_per_job", *cap)]),
        }
    }
}

fn check(doc: &Value, name: &str, edit: &CheckEdit) -> Result<Vec<Op>, NotAmended> {
    let at = |key: &'static str| ["checks", name, key];
    Ok(match edit {
        CheckEdit::Add(new) => {
            declared(doc, "checks", name, false)?;
            vec![Op::set(&["checks", name], new.node())]
        }
        CheckEdit::Remove => vec![removal(doc, "checks", name)?],
        other => {
            declared(doc, "checks", name, true)?;
            vec![match other {
                CheckEdit::Run(run) => Op::set(&at("run"), text(run)),
                CheckEdit::Requires(names) => listed(&at("requires"), names),
                CheckEdit::When(patterns) => listed(&at("when"), patterns),
                CheckEdit::Narrow(Some(narrow)) => Op::set(&at("narrow"), narrow.node()),
                CheckEdit::Narrow(None) => Op::remove(&at("narrow")),
                CheckEdit::Add(_) | CheckEdit::Remove => unreachable!("matched above"),
            }]
        }
    })
}

fn command(doc: &Value, name: &str, edit: &CommandEdit) -> Result<Vec<Op>, NotAmended> {
    let at = |key: &'static str| ["commands", name, key];
    Ok(match edit {
        CommandEdit::Add(new) => {
            declared(doc, "commands", name, false)?;
            vec![Op::set(&["commands", name], new.node())]
        }
        CommandEdit::Remove => vec![removal(doc, "commands", name)?],
        other => {
            declared(doc, "commands", name, true)?;
            match other {
                CommandEdit::Run(run) => vec![optional(&at("run"), run.as_deref())],
                CommandEdit::Serve(serve) => vec![optional(&at("serve"), serve.as_deref())],
                CommandEdit::Ready(ready) => vec![optional(&at("ready"), ready.as_deref())],
                CommandEdit::Links(links) => vec![match links.is_empty() {
                    true => Op::remove(&at("links")),
                    false => Op::set(&at("links"), links_node(links)),
                }],
                // Absent already means `false`, so clearing the flag takes out a
                // `true` and leaves a `false` somebody wrote on purpose.
                CommandEdit::Destructive(true) => {
                    vec![Op::set(&at("destructive"), Node::Flag(true))]
                }
                CommandEdit::Destructive(false) => match found(doc, &at("destructive")) {
                    Some(Value::Bool(true)) => vec![Op::remove(&at("destructive"))],
                    _ => Vec::new(),
                },
                CommandEdit::Add(_) | CommandEdit::Remove => unreachable!("matched above"),
            }
        }
    })
}

fn port(doc: &Value, name: &str, edit: &PortEdit) -> Result<Vec<Op>, NotAmended> {
    Ok(match edit {
        PortEdit::Add(new) => {
            declared(doc, "ports", name, false)?;
            vec![Op::set(&["ports", name], new.node())]
        }
        PortEdit::Remove => vec![removal(doc, "ports", name)?],
        PortEdit::Container(container) => {
            declared(doc, "ports", name, true)?;
            vec![port_key(doc, name, "container", container.map(number))]
        }
        PortEdit::Env(env) => {
            declared(doc, "ports", name, true)?;
            vec![port_key(doc, name, "env", env.as_deref().map(text))]
        }
    })
}

/// A port with nothing left under it is `{}` — a port Armada places — rather
/// than a key with no value, which would not load.
fn port_key(doc: &Value, name: &str, key: &str, to: Option<Node>) -> Op {
    let at = ["ports", name, key];
    match to {
        Some(node) => Op::set(&at, node),
        None if only_key(doc, &["ports", name], key) => {
            Op::set(&["ports", name], Node::Map(Vec::new()))
        }
        None => Op::remove(&at),
    }
}

fn policy(key: &str, word: Option<&str>) -> Op {
    optional(&[key], word)
}

/// A `drone:` holding nothing is refused, so removing its last key removes it.
fn dial(doc: &Value, key: &str, cap: Option<u32>) -> Op {
    match cap {
        Some(cap) => Op::set(&["drone", key], number(cap)),
        None if only_key(doc, &["drone"], key) => Op::remove(&["drone"]),
        None => Op::remove(&["drone", key]),
    }
}

/// Removing a registry's last entry removes the registry, for [`dial`]'s
/// reason one section over. **Either way the comment block directly above
/// what goes, goes with it** — the owner's rule, Journey 9, *Editing*.
fn removal(doc: &Value, registry: &str, name: &str) -> Result<Op, NotAmended> {
    declared(doc, registry, name, true)?;
    Ok(match only_key(doc, &[registry], name) {
        true => Op::remove_entry(&[registry]),
        false => Op::remove_entry(&[registry, name]),
    })
}

/// Refuses where `name` is not as `wanted` says it should be.
fn declared(doc: &Value, registry: &str, name: &str, wanted: bool) -> Result<(), NotAmended> {
    match holds(doc, &[registry, name]) == wanted {
        true => Ok(()),
        false => Err(NotAmended::Misnamed {
            key: format!("{registry}.{name}"),
            declared: !wanted,
        }),
    }
}

fn found<'a>(doc: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter()
        .try_fold(doc, |value, key| value.as_mapping()?.get(*key))
}

fn holds(doc: &Value, path: &[&str]) -> bool {
    found(doc, path).is_some()
}

/// Whether the map at `path` holds `key` and nothing else.
fn only_key(doc: &Value, path: &[&str], key: &str) -> bool {
    found(doc, path)
        .and_then(Value::as_mapping)
        .is_some_and(|map: &Mapping| map.len() == 1 && map.contains_key(key))
}

fn optional(path: &[&str], value: Option<&str>) -> Op {
    match value {
        Some(value) => Op::set(path, text(value)),
        None => Op::remove(path),
    }
}

fn listed(path: &[&str], items: &[String]) -> Op {
    match items.is_empty() {
        true => Op::remove(path),
        false => Op::set(path, texts(items)),
    }
}

fn text(value: &str) -> Node {
    Node::Text(value.to_string())
}

fn texts(items: &[String]) -> Node {
    Node::List(items.iter().map(|item| text(item)).collect())
}

fn number(value: u32) -> Node {
    Node::Number(Number::from(u64::from(value)))
}

fn links_node(links: &[NewLink]) -> Node {
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
    fn node(&self) -> Node {
        Entries::new()
            .with("run", Some(text(&self.run)))
            .list("requires", &self.requires)
            .list("when", &self.when)
            .with("narrow", self.narrow.as_ref().map(NewNarrowing::node))
            .done()
    }
}

impl NewNarrowing {
    fn node(&self) -> Node {
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
    fn node(&self) -> Node {
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
    fn node(&self) -> Node {
        Entries::new()
            .with("container", self.container.map(number))
            .with("env", self.env.as_deref().map(text))
            .done()
    }
}
