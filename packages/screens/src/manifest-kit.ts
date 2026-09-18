// Kit's MCP servers, read for the Manifest surface and changed there. #1275.
//
// Beside `manifest-allows.ts`, whose shape this is: the wire half and the hook
// that draws from it, so main can import the read type without a component in
// reach. One type for every act — each answers with the whole list, so a caller
// folding one answer has already folded the other.

import { useEffect, useState } from "react";
import type {
  AddKitServer,
  KitInventory,
  KitServers,
  ManifestReach,
  Outcome,
  ReachesDrones,
  ServerAddress,
} from "@armada/protocol";

/** `GET /kit/servers` and the four acts under it, read into the app. */
export type KitServersRead = { ok: true; kit: KitServers } | { ok: false; outcome: Outcome };

/** `GET /kit/inventory`, read into the app. #1491. */
export type KitInventoryRead = { ok: true; setup: KitInventory } | { ok: false; outcome: Outcome };

/** What the Manifest surface asks of the host for Kit. */
export type KitSlice = {
  /** The setup a person already has, read to be shown. Machine-wide. #1491. */
  onReadKitInventory: () => Promise<KitInventoryRead>;
  onListKitServers: () => Promise<KitServersRead>;
  onAddKitServer: (adding: AddKitServer) => Promise<KitServersRead>;
  onForgetKitServer: (name: string) => Promise<KitServersRead>;
  onSetKitServerReach: (name: string, drones: ReachesDrones) => Promise<KitServersRead>;
  onSetManifestServerReach: (name: string, reach: ManifestReach | null) => Promise<KitServersRead>;
};

/**
 * An address a person typed, turned into the shape the wire carries.
 *
 * **A program's arguments are split on whitespace here and nowhere else.**
 * There is no shell between Bridge and the spawn, so something has to do it,
 * and doing it once beside the form is what keeps the field a person reads and
 * the array fleet stores in step. `null` where the text carries neither.
 */
export function addressTyped(transport: "stdio" | "http", typed: string): ServerAddress | null {
  const text = typed.trim();
  if (text === "") return null;
  if (transport === "http") {
    return text.startsWith("http://") || text.startsWith("https://") ? { transport, url: text } : null;
  }
  const [command, ...args] = text.split(/\s+/);
  return command === undefined ? null : { transport, command, args };
}

/** An address, as a row draws it: the program and its arguments, or the URL. */
export function addressReads(address: ServerAddress): string {
  return address.transport === "http" ? address.url : [address.command, ...address.args].join(" ");
}

/**
 * Kit's servers, read once on opening and again off whatever an act answers
 * with.
 *
 * **`undefined` until the first read lands**, `useRepositoryAllows`' reason: an
 * empty list drawn before the read would say Kit holds nothing, which is the
 * one answer on this page nobody should be given by accident.
 */
export function useKit(slice: KitSlice): {
  setup: KitInventory | undefined;
  kit: KitServers | undefined;
  refused: Outcome | null;
  onAdd: (adding: AddKitServer) => void;
  onForget: (name: string) => void;
  onKitReach: (name: string, drones: ReachesDrones) => void;
  onManifestReach: (name: string, reach: ManifestReach | null) => void;
} {
  const [kit, setKit] = useState<KitServers | undefined>(undefined);
  const [setup, setSetup] = useState<KitInventory | undefined>(undefined);
  const [refused, setRefused] = useState<Outcome | null>(null);

  const took = (read: KitServersRead): void => {
    if (read.ok) {
      setKit(read.kit);
      setRefused(null);
      return;
    }
    // The `Outcome` and not a sentence. `copy.ts` reaches components, and a
    // value import from here would pull every `.tsx` that package exports into
    // main and preload's `--jsx`-less projects — `manifest-allows.ts`'s rule.
    setRefused(read.outcome);
  };

  useEffect(() => {
    void slice.onListKitServers().then((read) => {
      if (read.ok) setKit(read.kit);
    });
    // Read fresh on opening rather than kept: a person who installs a skill and
    // comes back expects to see it, and nothing in the app is told when they do.
    void slice.onReadKitInventory().then((read) => {
      if (read.ok) setSetup(read.setup);
    });
    // Once, on opening. Every act below folds its own answer.
  }, []);

  return {
    setup,
    kit,
    refused,
    onAdd: (adding) => void slice.onAddKitServer(adding).then(took),
    onForget: (name) => void slice.onForgetKitServer(name).then(took),
    onKitReach: (name, drones) => void slice.onSetKitServerReach(name, drones).then(took),
    onManifestReach: (name, reach) => void slice.onSetManifestServerReach(name, reach).then(took),
  };
}
