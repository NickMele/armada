// The app's Manifest surface, in the window Bridge draws it in.
//
// **Not a drawing of the surface — the surface.** `Shell`, `headOf` and
// `Manifest` are the three the app renders, called the way `App` calls them,
// so the rail, the head and every row come out of the app's own code. What is
// made up is only the data, and it is typed against the wire.

import { connectedTo, PROTOCOL_VERSION, type Connection } from "@armada/protocol";
import type { CheckoutRunList, Outcome, RunOutputRead } from "@armada/protocol";
import { headOf, Shell, statementOf, SURFACE } from "@armada/shell";
import { Manifest } from "../../../Manifest";
import type { CheckoutRunFollowed, CheckoutRunSheetRead } from "../../../checkout-runs";
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

export function ManifestFrom({
  sheet,
  followed = { state: "none" },
  runs = { runs: [], unreadable: [] },
  now = NOW,
}: {
  sheet: CheckoutRunSheetRead;
  followed?: CheckoutRunFollowed;
  /** What `list_checkout_runs` answers — *Earlier runs*, and what Undo acts on. */
  runs?: CheckoutRunList;
  now?: number;
}) {
  const statement = statementOf(CONNECTED, now, now);
  const head = headOf({
    reading: false,
    composing: false,
    auditing: false,
    clearing: false,
    manifest: true,
    live: true,
    refreshing: false,
    onCloseComposer: noop,
    onCompose: noop,
    onCloseReports: noop,
    onReadReports: noop,
    onCloseWorktrees: noop,
    onReadWorktrees: noop,
    onCloseManifest: noop,
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
