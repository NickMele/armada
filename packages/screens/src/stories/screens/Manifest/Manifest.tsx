// The app's Manifest surface, in the window Bridge draws it in.
//
// **Not a drawing of the surface — the surface.** `Shell`, `headOf` and
// `Manifest` are the three the app renders, called the way `App` calls them,
// so the rail, the head and every row come out of the app's own code. What is
// made up is only the data, and it is typed against the wire.

import { useRef, useState } from "react";
import { connectedTo, PROTOCOL_VERSION, type Connection } from "@armada/protocol";
import type {
  CheckoutRunFollowed,
  CheckoutRunList,
  CheckoutRunSheetRead,
  ManifestReading,
  Outcome,
  RunOutputRead,
  SaveManifestFile,
} from "@armada/protocol";
import { headOf, Shell, statementOf, SURFACE } from "@armada/shell";
import { Manifest } from "../../../Manifest";
import type { ManifestSaveAnswer, ManifestView } from "../../../editing";
import { useManifestEditing } from "../../../manifest-file";
import { CREATED_AT, manifest, MANIFEST_ID } from "../../../fixtures/build/base";

/** The moment every elapsed figure on this page is read at, so it never moves. */
export const NOW = Date.parse("2026-09-12T14:20:00Z");

/** A Fleet that answered, as Bridge holds one once the socket is up. */
const CONNECTED: Connection = connectedTo(
  { protocolVersion: PROTOCOL_VERSION, pid: 4242, port: 7878, startedAt: CREATED_AT },
  1,
);

const noop = () => {};
const nothingHappens = (): Promise<Outcome> => Promise.resolve({ ok: true });

/** Where Fleet resolved the Manifest, in the shape `root.join("armada.yml")` gives. */
export const MANIFEST_PATH = "/Users/user/armada/armada.yml";

/** A cut of this repository's own `armada.yml`, as `GET /manifest/file` answers it. */
export const MANIFEST_TEXT = `version: 1
id: armada
base: main

drone:
  poke_limit: 3

checks:
  build:
    run: cargo build --workspace --locked
  typecheck:
    run: pnpm typecheck
`;

/** What a pull brought in while the edit was open. */
export const PULLED_TEXT = MANIFEST_TEXT.replace("base: main", "base: main\n\nauto_merge: never");

/**
 * What a save comes to, played the way Fleet plays it: the write answers,
 * then the watch re-reads and publishes. `refused` is a correction Fleet would
 * not adopt; `moved` is a pull that landed under the edit.
 */
export type SaveGoesTo = "took" | "refused" | "moved";

/** The instant a save in a story lands, and the reading that follows it. */
const WROTE_AT = "2026-09-12T14:20:03.120Z";
const READ_AT = "2026-09-12T14:20:04.700Z";

export function ManifestFrom({
  sheet,
  followed = { state: "none" },
  runs = { runs: [], unreadable: [] },
  now = NOW,
  view = "run",
  save = "took",
}: {
  sheet: CheckoutRunSheetRead;
  followed?: CheckoutRunFollowed;
  /** What `list_checkout_runs` answers — *Earlier runs*, and what Undo acts on. */
  runs?: CheckoutRunList;
  now?: number;
  /** Which view the surface opens on. */
  view?: ManifestView;
  /** What pressing Save comes to. */
  save?: SaveGoesTo;
}) {
  // A disk and a watch, faked just far enough to be Fleet's: a read answers
  // what is on disk, a save compares against it, and a reading follows.
  const disk = useRef(MANIFEST_TEXT);
  const [reading, setReading] = useState<ManifestReading | null>(null);
  const editing = useManifestEditing({
    showing: true,
    reading,
    initialView: view,
    onReadFile: () =>
      Promise.resolve({ ok: true, file: { path: MANIFEST_PATH, text: disk.current } }),
    onSaveFile: (body: SaveManifestFile): Promise<ManifestSaveAnswer> => {
      if (save === "moved") {
        disk.current = PULLED_TEXT;
        return Promise.resolve({ state: "moved", onDisk: PULLED_TEXT });
      }
      disk.current = body.text;
      setReading(
        save === "refused"
          ? {
              path: MANIFEST_PATH,
              at: READ_AT,
              refused: {
                summary: "armada.yml could not be adopted: 1 fault",
                faults: [{ key: "drone.poke_limit", fault: "invalid type: string, expected u32" }],
              },
            }
          : {
              path: MANIFEST_PATH,
              at: READ_AT,
              moved: [{ key: "drone.poke_limit", before: "3", after: "5" }],
            },
      );
      return Promise.resolve({ state: "saved", saved: { path: MANIFEST_PATH, at: WROTE_AT } });
    },
  });
  const statement = statementOf(CONNECTED, now, now);
  const head = headOf({
    reading: false,
    composing: false,
    auditing: false,
    clearing: false,
    // The view the surface is showing, as `App` passes it.
    manifest: editing.view,
    live: true,
    refreshing: false,
    onCloseComposer: noop,
    onCompose: noop,
    onCloseReports: noop,
    onReadReports: noop,
    onCloseWorktrees: noop,
    onReadWorktrees: noop,
    onRefresh: noop,
    jobs: [],
    onClearTerminal: noop,
    onForgetTerminal: noop,
  });
  return (
    <div style={{ height: "100vh", display: "flex", flexDirection: "column" }}>
      <Shell
        connection={CONNECTED}
        statement={statement}
        manifests={[manifest()]}
        scope={MANIFEST_ID}
        onScope={noop}
        jobs={[]}
        capacity={{ bound: 4, occupied: 0 }}
        title={head?.title}
        summary={head?.summary}
        actions={head?.actions}
        showing={SURFACE.manifest}
      >
        <div className="armada-screen__mounted">
          <Manifest
            sheet={sheet}
            followed={followed}
            picked={null}
            now={now}
            editing={editing}
            onSaid={noop}
            onObserveRun={noop}
            onStartRun={nothingHappens}
            onStopRun={nothingHappens}
            onUndoRun={nothingHappens}
            onListRuns={() => Promise.resolve({ ok: true, runs })}
            onGetRunOutput={(): Promise<RunOutputRead> =>
              Promise.resolve({ ok: false, outcome: { ok: false, why: "not_connected" } })
            }
            onStartServer={nothingHappens}
            onStopServer={nothingHappens}
            onOpenServerLink={() => Promise.resolve({ ok: true })}
          />
        </div>
      </Shell>
    </div>
  );
}
