//! A Command that stays running — `serve`, and the three keys that go with it.
//! `docs/concepts/manifest.md`, *Commands that keep running*.
//!
//! **A second type, not a flag on [`Command`].** A server never exits, so
//! anything that waits on a Command — `setup.requires`, a Check's `requires`,
//! a Drone's `Bash` grant — would wait forever on one. Keeping servers out of
//! the map those readers walk makes that call unspeakable rather than refused
//! at each of them: `Manifest::command` cannot answer with a server.
//!
//! Both are written under `commands:`, because to a person they are both
//! things the repository can run; [`entry`] is where one becomes the other.

use serde_yaml_ng::Value;

use super::Command;
use crate::error::{Fault, Refusal};
use crate::yaml::{self, Table};

/// The keys read inside one `links` entry.
const LINK_KEYS: &[&str] = &["url", "name"];

/// A Command declaring `serve`: started, and held until something stops it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Server {
    pub(super) run: Option<String>,
    pub(super) serve: String,
    pub(super) ready: Option<String>,
    pub(super) links: Vec<Link>,
    pub(super) destructive: bool,
}

impl Server {
    /// What runs first. The server starts only if it exits zero. `None` where
    /// the file declares no `run`, which is a server with nothing to build.
    pub fn run(&self) -> Option<&str> {
        self.run.as_deref()
    }

    /// What starts the server, verbatim — `${port.NAME}` unresolved.
    pub fn serve(&self) -> &str {
        &self.serve
    }

    /// A command that exits zero once the server answers. **`None` is serving
    /// once started**, never "never ready" — and never a duration either.
    pub fn ready(&self) -> Option<&str> {
        self.ready.as_deref()
    }

    /// The addresses offered as buttons, in the order the file wrote them.
    pub fn links(&self) -> &[Link] {
        &self.links
    }

    /// Whether a **Drone** starting this needs a person. A person's own start
    /// needs no second approval, as for any Command.
    pub fn is_destructive(&self) -> bool {
        self.destructive
    }
}

/// One address a server offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub(super) url: String,
    pub(super) name: Option<String>,
}

impl Link {
    /// Verbatim — `${port.NAME}` unresolved.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// The button's label. `None` draws the URL itself.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

/// What one `commands.<name>` entry is.
pub(super) enum CommandEntry {
    Runs(Command),
    Serves(Server),
}

/// Read one `commands.<name>`. **`serve` decides which it is**: without it,
/// `run` is required and `ready` and `links` are refused, because a Command
/// that exits has no answer to wait for and no address to offer.
pub(super) fn entry(
    at: &str,
    value: &Value,
    known: &'static [&'static str],
    out: &mut Vec<Refusal>,
) -> Option<CommandEntry> {
    let mut table = Table::open(at, value, out)?;
    let serve = table
        .optional("serve")
        .and_then(|value| yaml::text(&table.at("serve"), value, out));
    let serves = table.present("serve");
    let run = match serves {
        true => table
            .optional("run")
            .and_then(|value| yaml::text(&table.at("run"), value, out)),
        false => table
            .required("run", out)
            .and_then(|value| yaml::text(&table.at("run"), value, out)),
    };
    let destructive = table
        .optional("destructive")
        .and_then(|value| yaml::flag(&table.at("destructive"), value, out))
        .unwrap_or(false);
    let ready = table
        .optional("ready")
        .and_then(|value| yaml::text(&table.at("ready"), value, out));
    let links = table
        .optional("links")
        .map(|value| links(&table.at("links"), value, out));
    if !serves {
        for key in ["ready", "links"] {
            if table.present(key) {
                out.push(Refusal::new(table.at(key), Fault::OnlyAServerReads));
            }
        }
    }
    table.close(known, out);
    if !serves {
        return Some(CommandEntry::Runs(Command {
            run: run?,
            destructive,
        }));
    }
    Some(CommandEntry::Serves(Server {
        run,
        serve: serve?,
        ready,
        links: links.unwrap_or_default(),
        destructive,
    }))
}

/// `links`, each a `url` and an optional `name`. One entry that will not read
/// is refused and the rest are kept, so one pass reports the whole list.
fn links(at: &str, value: &Value, out: &mut Vec<Refusal>) -> Vec<Link> {
    let Some(items) = yaml::list(at, value, out) else {
        return Vec::new();
    };
    let mut read = Vec::with_capacity(items.len());
    for (key, item) in items {
        let Some(mut table) = Table::open(&key, item, out) else {
            continue;
        };
        let url = table
            .required("url", out)
            .and_then(|value| yaml::text(&table.at("url"), value, out));
        let name = table
            .optional("name")
            .and_then(|value| yaml::text(&table.at("name"), value, out));
        table.close(LINK_KEYS, out);
        if let Some(url) = url {
            read.push(Link { url, name });
        }
    }
    read
}
