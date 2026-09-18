// Kit's MCP servers, and each manifest's word over one. Since protocol 15.2.
// `crates/ipc/src/kit.rs`, #1275.
//
// `resolves` crosses rather than being worked out here: it is fleet's own
// resolution over the two tiers, and the same answer decides the document a
// drone is spawned against. A surface that recomputed it could draw a server
// as reaching a drone that no drone is handed.

/**
 * Where a server is, in the two shapes the agent CLI's schema accepts. Tagged
 * by `transport`, so a reader matches one field rather than guessing from
 * which of two optional keys turned up.
 */
export type ServerAddress =
  | { transport: "stdio"; command: string; args: string[] }
  | { transport: "http"; url: string };

/** Kit's own tier for a server: whether a drone gets it where the manifest is silent. */
export type ReachesDrones = "no" | "yes";

/**
 * What one manifest says about one of kit's servers. Absent where it has said
 * nothing, which is kit's default answering.
 */
export type ManifestReach = "extended" | "restricted";

/** One row of `get_kit_servers`. */
export type KitServerRow = {
  name: string;
  address: ServerAddress;
  /** Kit's own default, across every manifest. */
  drones: ReachesDrones;
  /** This manifest's word, absent where it has none. */
  manifest?: ManifestReach;
  /** Whether a drone dispatched here gets it. The resolution, not a hint. */
  resolves: boolean;
  /** When it was added, by fleet's clock. */
  added_at: string;
  /** Who added it, in the envelope's spelling. Left as `string` like `actor`. */
  by: string;
};

/** `get_kit_servers`' answer, and what every act on kit answers with. */
export type KitServers = {
  servers: KitServerRow[];
};

/** The body of `add_kit_server`. A name kit holds replaces that server's address. */
export type AddKitServer = {
  name: string;
  address: ServerAddress;
};

/** The body of `forget_kit_server`. A name kit does not hold is a 409. */
export type ForgetKitServer = {
  name: string;
};

/** The body of `set_kit_server_reach` — kit's own tier. */
export type SetKitServerReach = {
  name: string;
  drones: ReachesDrones;
};

/**
 * The body of `set_manifest_server_reach` — the manifest's own tier.
 *
 * **`null` is sent, never implied.** Fleet refuses a body with no `reach` key
 * rather than reading it as a take-back, `SetModel`'s reason: a bridge that
 * dropped the field would throw away a person's word about who may reach a
 * server and get a 200 for it.
 */
export type SetManifestServerReach = {
  name: string;
  reach: ManifestReach | null;
};

// The setup a person already works with, read from their agent harness's own
// home and shown by kind. Since protocol 17.2. `crates/ipc/src/kit.rs`, #1491.
//
// **No address on any of it.** A row names a thing and says where it came from,
// and there is nothing here a server a drone gets could be built out of —
// reading is not granting, and allowing stays its own act on a kit row.

/** One thing a person already has. */
export type SetupItem = {
  name: string;
  /** Its own words for itself, where its file carries them. */
  says?: string;
  /** Where it came from, as a person would type it. */
  source: string;
};

/** Something of the right shape in the right place that would not read. */
export type SetupUnreadable = {
  source: string;
  why: string;
};

/**
 * What became of one kind. **Tagged**, so an empty list is *you have none of
 * these* and never *nothing looked*.
 */
export type WhatWasRead =
  | { what: "read"; items: SetupItem[]; unreadable: SetupUnreadable[] }
  | { what: "not_read"; why: string };

/** Armada's word for one kind of thing, never a harness's. */
export type SetupKind =
  | "skills"
  | "plugins"
  | "agent_file"
  | "sub_agents"
  | "commands"
  | "mcp_servers"
  | "allowlist"
  | "models";

/** One kind, and what became of it. */
export type SetupKindRow = {
  kind: SetupKind;
  read: WhatWasRead;
};

/** `get_kit_inventory`'s answer. */
export type KitInventory = {
  /**
   * The harness, in its own name. **Drawn, never matched on** — it arrives as
   * data an adapter produced, which is what lets a second harness draw here
   * without this file changing.
   */
  harness: string;
  home: string;
  /** Whether that home is there at all. */
  present: boolean;
  /** Every kind, in a fixed order, including the ones nothing reads yet. */
  kinds: SetupKindRow[];
};
