// The Kit surface — the MCP servers a person has connected, and which of them
// a Drone dispatched against the picked repository is handed. #1275.
//
// **A rail surface and not a Manifest tab.** It was a tab first; the owner
// reversed it on 18 Sep, because Kit is his and not a repository's and he would
// rather move a digit than reach a machine-wide thing through one repository's
// screen. Kit took `⌘8` and Settings moved to `⌘9`.
//
// **Both tiers stay on one screen**, because `resolves` is the answer over the
// two and a surface showing one of them cannot say why a Drone gets a server.
//
// **What a person already has is read first and drawn first.** #1491: this
// screen opened on *Nothing in your Kit yet* while fourteen skills and eight
// plugins sat one directory away. What is read reaches no Drone — the servers
// below are the only ones that can, and only once allowed.

// Which repository "here" means is the rail's pick, the same pick every other
// per-repository read follows. On All repositories there is no Manifest to
// narrow, so the app asks for one rather than drawing a dead control — the
// Manifest surface's own arrangement.
//
// **The ask replaces the servers and not the screen.** What a person already
// has is this machine's and answers for every repository, so it is drawn with
// no pick at all; it is the second tier that needs one.

import type { ReactNode } from "react";

import { KitServers, KitSetup } from "@armada/components";

import { said } from "./copy";
import { addressReads, addressTyped, useKit, type KitSlice } from "./manifest-kit";

export type KitProps = KitSlice & {
  /**
   * The repository the rail has picked, as a person reads it. Names the second
   * tier. `null` is All repositories, and draws [`ask`](KitProps.ask) in the
   * servers' place.
   */
  repository: string | null;
  /** What to draw where the servers would be, with no repository picked. */
  ask?: ReactNode;
};

export function Kit(props: KitProps) {
  const kit = useKit(props);
  return (
    <div className="armada-kit">
      <KitSetup setup={kit.setup} />
      {props.repository === null ? (
        props.ask
      ) : (
        <KitServers
          here={props.repository}
          servers={kit.kit?.servers.map((server) => ({
            name: server.name,
            address: addressReads(server.address),
            kind: server.address.transport,
            reachesByDefault: server.drones === "yes",
            here: server.manifest,
            resolves: server.resolves,
          }))}
          refused={kit.refused === null ? undefined : said(kit.refused)}
          onAdd={({ name, kind, address }) => {
            const typed = addressTyped(kind, address);
            // Fleet refuses the same shapes and says why. What is refused here is
            // an empty field, which has nothing to send and nothing to say.
            if (typed !== null) kit.onAdd({ name, address: typed });
          }}
          onForget={kit.onForget}
          onKitReach={(name, reaches) => kit.onKitReach(name, reaches ? "yes" : "no")}
          onHereReach={kit.onManifestReach}
        />
      )}
    </div>
  );
}
