//! Kit's MCP servers, and what each Manifest says about one — `#1275`.
//!
//! **Kit's home is `~/.armada` and this is not a contradiction.** That folder
//! is what a person edits and syncs; a server is a record with a reach a Drone
//! is spawned against, and `crate::manifest_allowed` is the precedent for
//! keeping exactly that kind of rule in Fleet's own store. Import and export —
//! `kit.md`'s own actions — are how it reaches the folder, and neither is built.
//!
//! **No event, for [`crate::manifest_allowed`]'s reason**: neither table moves
//! a status or a step, so each is the authority for its own field and
//! [`crate::read`] folds nothing from it.

use core_model::{
    Actor, KitServer, ManifestId, ManifestReach, ReachesDrones, ServerAddress, ServerName,
    Timestamp,
};

use crate::columns::{read_server_args, write_server_args};
use crate::error::{fault, RowError, WriteError};
use crate::open::Store;
use crate::row::{column, enum_value};

/// Version 80 — the servers a person put in their Kit, and each Manifest's
/// word over one.
///
/// **`name` is the primary key of the first table** because it is the key in
/// the document a Drone is spawned against: two rows under one name could not
/// both be written there, so a second add replaces rather than joins.
///
/// **`transport` decides which of `command`/`args` and `url` carries the
/// address**, and the other is empty. Two nullable shapes in one row would let
/// a reader meet a server that is neither.
///
/// The second table cascades: a Manifest's word about a server Kit no longer
/// holds is a rule about nothing, and leaving it would let a re-added name
/// arrive already extended.
pub(crate) const V80: &str = r#"
CREATE TABLE kit_mcp_servers (
    name      TEXT NOT NULL PRIMARY KEY,
    transport TEXT NOT NULL,
    command   TEXT NOT NULL,
    args      TEXT NOT NULL,
    url       TEXT NOT NULL,
    drones    TEXT NOT NULL,
    added_at  TEXT NOT NULL,
    "by"      TEXT NOT NULL
) STRICT;

CREATE TABLE manifest_mcp_servers (
    manifest_id TEXT NOT NULL,
    name        TEXT NOT NULL REFERENCES kit_mcp_servers(name) ON DELETE CASCADE,
    reach       TEXT NOT NULL,
    set_at      TEXT NOT NULL,
    "by"        TEXT NOT NULL,
    PRIMARY KEY (manifest_id, name)
) STRICT;
"#;

impl Store {
    /// Every server in Kit, by name.
    ///
    /// Ordered by name rather than by when it was added: this is a set a
    /// person scans for one row, and the order a list is read in is the order
    /// they will look in.
    pub fn kit_servers(&self) -> Result<Vec<KitServer>, RowError> {
        let mut asking = self
            .conn
            .prepare(
                "SELECT name, transport, command, args, url, drones, added_at, \"by\" \
                 FROM kit_mcp_servers ORDER BY name",
            )
            .map_err(unreadable)?;
        let rows = asking
            .query_map([], |row| Ok(read_server(row)))
            .map_err(unreadable)?;
        let mut servers = Vec::new();
        for row in rows {
            servers.push(row.map_err(unreadable)??);
        }
        Ok(servers)
    }

    /// Put a server in Kit, replacing one of the same name.
    ///
    /// **A replacement keeps the reach the name already had**, at both tiers:
    /// `drones` is left as it was and the Manifest rows cascade off nothing.
    /// Correcting a typo in a command is not a reason to make a person allow
    /// the server again — and it cannot widen anything either, because the
    /// column is not written here.
    pub fn put_kit_server(&mut self, server: &KitServer) -> Result<(), WriteError> {
        let (command, args, url) = match &server.address {
            ServerAddress::Stdio { command, args } => {
                (command.as_str(), write_server_args(args), "")
            }
            ServerAddress::Http { url } => ("", write_server_args(&[]), url.as_str()),
        };
        self.conn
            .execute(
                "INSERT INTO kit_mcp_servers \
                 (name, transport, command, args, url, drones, added_at, \"by\") \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
                 ON CONFLICT (name) DO UPDATE SET \
                 transport = ?2, command = ?3, args = ?4, url = ?5",
                (
                    server.name.as_str(),
                    server.address.transport(),
                    command,
                    args,
                    url,
                    server.drones.as_wire(),
                    server.added_at.as_str(),
                    server.by.as_wire(),
                ),
            )
            .map_err(fault("putting a server in Kit"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// Take a server out of Kit. `true` where a row went.
    ///
    /// Every Manifest's word about it goes with it, by the cascade above.
    pub fn forget_kit_server(&mut self, name: &ServerName) -> Result<bool, WriteError> {
        let removed = self
            .conn
            .execute(
                "DELETE FROM kit_mcp_servers WHERE name = ?1",
                [name.as_str()],
            )
            .map_err(fault("taking a server out of Kit"))
            .map_err(WriteError::Database)?;
        Ok(removed > 0)
    }

    /// Set Kit's own default for a server. `true` where a row moved.
    pub fn set_kit_server_reach(
        &mut self,
        name: &ServerName,
        drones: ReachesDrones,
    ) -> Result<bool, WriteError> {
        let moved = self
            .conn
            .execute(
                "UPDATE kit_mcp_servers SET drones = ?2 WHERE name = ?1",
                (name.as_str(), drones.as_wire()),
            )
            .map_err(fault("setting Kit's default reach for a server"))
            .map_err(WriteError::Database)?;
        Ok(moved > 0)
    }

    /// Every word this Manifest has said about a Kit server, by name.
    pub fn manifest_server_reaches(
        &self,
        manifest_id: &ManifestId,
    ) -> Result<Vec<(ServerName, ManifestReach)>, RowError> {
        let mut asking = self
            .conn
            .prepare(
                "SELECT name, reach FROM manifest_mcp_servers \
                 WHERE manifest_id = ?1 ORDER BY name",
            )
            .map_err(unreadable)?;
        let rows = asking
            .query_map([manifest_id.as_str()], |row| Ok(read_reach(row)))
            .map_err(unreadable)?;
        let mut reaches = Vec::new();
        for row in rows {
            reaches.push(row.map_err(unreadable)??);
        }
        Ok(reaches)
    }

    /// Say what this Manifest does with a Kit server, or take the word back.
    ///
    /// `None` deletes the row, which is what leaves Kit's own default
    /// answering again — the absence `core_model::a_drone_resolves` reads.
    pub fn set_manifest_server_reach(
        &mut self,
        manifest_id: &ManifestId,
        name: &ServerName,
        reach: Option<ManifestReach>,
        at: &Timestamp,
        by: Actor,
    ) -> Result<(), WriteError> {
        match reach {
            None => {
                self.conn
                    .execute(
                        "DELETE FROM manifest_mcp_servers WHERE manifest_id = ?1 AND name = ?2",
                        (manifest_id.as_str(), name.as_str()),
                    )
                    .map_err(fault("taking back a Manifest's word about a server"))
                    .map_err(WriteError::Database)?;
            }
            Some(reach) => {
                self.conn
                    .execute(
                        "INSERT INTO manifest_mcp_servers \
                         (manifest_id, name, reach, set_at, \"by\") VALUES (?1, ?2, ?3, ?4, ?5) \
                         ON CONFLICT (manifest_id, name) DO UPDATE SET \
                         reach = ?3, set_at = ?4, \"by\" = ?5",
                        (
                            manifest_id.as_str(),
                            name.as_str(),
                            reach.as_wire(),
                            at.as_str(),
                            by.as_wire(),
                        ),
                    )
                    .map_err(fault("recording a Manifest's word about a server"))
                    .map_err(WriteError::Database)?;
            }
        }
        Ok(())
    }
}

/// One Kit server. A row whose `transport` is neither spelling, or whose
/// address column is empty for the transport it names, is a malformed row and
/// says so rather than reading back as a server pointing nowhere.
fn read_server(row: &rusqlite::Row<'_>) -> Result<KitServer, RowError> {
    let text = |name: &'static str| -> Result<String, RowError> {
        row.get(name).map_err(column(SERVERS, name))
    };
    let transport = text("transport")?;
    let address = match transport.as_str() {
        "stdio" => {
            let args =
                read_server_args(&text("args")?).map_err(|detail| RowError::MalformedColumn {
                    table: SERVERS,
                    column: "args",
                    detail,
                })?;
            ServerAddress::program(&text("command")?, &args)
        }
        "http" => ServerAddress::address(&text("url")?),
        _ => None,
    }
    .ok_or_else(|| RowError::MalformedColumn {
        table: SERVERS,
        column: "transport",
        detail: format!("`{transport}` names no address this build can write"),
    })?;
    Ok(KitServer {
        name: named(&text("name")?)?,
        address,
        drones: enum_value(
            ReachesDrones::from_wire,
            SERVERS,
            "drones",
            &text("drones")?,
        )?,
        added_at: Timestamp::from_rfc3339(text("added_at")?),
        by: enum_value(Actor::from_wire, SERVERS, "by", &text("by")?)?,
    })
}

fn read_reach(row: &rusqlite::Row<'_>) -> Result<(ServerName, ManifestReach), RowError> {
    let text = |name: &'static str| -> Result<String, RowError> {
        row.get(name).map_err(column(REACHES, name))
    };
    Ok((
        named(&text("name")?)?,
        enum_value(ManifestReach::from_wire, REACHES, "reach", &text("reach")?)?,
    ))
}

/// A stored name read back through the same constructor a person's goes
/// through, so a row written before the rule tightened is malformed rather than
/// a name no document can carry.
fn named(text: &str) -> Result<ServerName, RowError> {
    ServerName::named(text).ok_or_else(|| RowError::MalformedColumn {
        table: SERVERS,
        column: "name",
        detail: format!("`{text}` is not a name a server may carry"),
    })
}

fn unreadable(cause: rusqlite::Error) -> RowError {
    RowError::Database(fault("reading Kit's servers")(cause))
}

const SERVERS: &str = "kit_mcp_servers";
const REACHES: &str = "manifest_mcp_servers";
