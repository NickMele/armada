// The Manifest surface — Journey 9's *Running one* and the file half of
// *Editing*, with no Job in existence.
//
// # Why this is a surface at all
//
// Most of what this page does was already sanctioned and had nowhere to
// happen. The Manifest concept page says a Command is invoked both by a Drone
// during a Job and by a person directly through Bridge, off the same named
// registry, and that a person's own invocation needs no second approval
// because they are the one triggering it. **Running a single Check on demand
// is the one new capability** — Checks were specified as Fleet's, and the only
// ad-hoc path was Verify's dry-run, which runs all of them.
//
// The rail has carried a Manifest row since it shipped, which is what gives
// this `⌘6` — `⌘5` until Studios joined the rail third (#1287). Until now it reached nothing.
//
// # Two views, and the toggle between them is named by the file's path
//
// Correcting a Check used to mean leaving Bridge for an editor, and finding
// out whether the correction parsed meant watching Fleet's console. The file
// view is `armada.yml` itself and the answer to a save beside it. **The toggle
// is the file's name, never its format** — the lexicon bans calling a
// Manifest by one.
//
// Journey 9 puts forms in front of the file, with the file behind that toggle:
// Edit is the forms, and the path is the file.
//
// # What this page is not
//
// **Not the run sheet widened.** A sheet is dismissed back to what you were
// doing; this is where you were going. `RunPage` carries the whole of that
// difference and `checkout-runs.ts` the reading behind it.
//
// **Not a verdict.** No run from here writes Evidence and no Check gets a
// stored pass or fail against it — a pass that counted for a Job from the same
// tree is the path around verification that v1 proved becomes the default
// path. The transcript is keepable; the judgement is not.
//
// # Drift and Verify, as two panels above the runner
//
// Journey 9's *Verify* answers two questions: drift, whether the file is still
// true, read on opening at no cost; and Verify, whether it still works, which
// runs setup and every Check behind its own button. **Two panels, never one**,
// and both report without offering a fix. `verify.ts` reads them.
//
// # What the forms do not carry yet
//
// `setup`, `evidence`, `after_merge` and `base`: no edit reaches them until the
// writer's own edits for them land, so they are the file view's for now.

import { useState, type ReactNode } from "react";
import type { ManifestDriftRead, Outcome } from "@armada/protocol";
import {
  Alert,
  Button,
  DriftPanel,
  KitServers,
  ManifestFile,
  ManifestForm,
  RunPage,
  TabPanel,
  Tabs,
  VerifyPanel,
} from "@armada/components";

import { said } from "./copy";
import { NOTHING_SERVED, servesNothing } from "./locate-reads";
import { useManifestRuns, type ManifestSlice } from "./checkout-runs";
import { useRepositoryAllows, type RepositoryAllowsSlice } from "./manifest-allows";
import { addressReads, addressTyped, useKit, type KitSlice } from "./manifest-kit";
import type { ManifestEditing } from "./manifest-file";
import type { ManifestForming } from "./manifest-form";
import { driftPanelOf, verifyPanelOf } from "./verify";

export type ManifestProps = ManifestSlice & RepositoryAllowsSlice & KitSlice & {
  /**
   * The entry the command palette picked, or `null`.
   *
   * **It selects; it does not run.** Journey 9 has the palette opening the
   * surface with a Check chosen, and a row that started a destructive Command
   * straight off a list of forty would be an act nobody confirmed.
   */
  picked: string | null;
  /** The app's one `now`, which every elapsed figure on screen is drawn from. */
  now: number;
  /** A sentence to say — today, only a server link that did not open. */
  onSaid: (sentence: string) => void;
  /** The file view, held by the app so an unsaved edit outlives the surface. */
  editing: ManifestEditing;
  /** The forms, held by the app for the same reason. */
  form: ManifestForming;
  /** `GET /manifest/drift`, held open by the app while this surface shows. */
  drift: ManifestDriftRead;
  /** Press Verify. **Only ever from its button** — nothing here calls it on a read. */
  onStartVerify: () => Promise<Outcome>;
  /** Setup, drawn by the app, and whether it is the view up. Absent leaves the tab out. */
  setup?: ReactNode;
  settingUp?: boolean;
  onSettingUp?: (on: boolean) => void;
  /** `false` where the picked repository has no `armada.yml` yet: Setup is the only view with anything to show. */
  setUp?: boolean;
};

/**
 * Everything this repository's Manifest declares, one press that runs one of
 * them in the checkout as it is on disk, and the file itself.
 *
 * **The read is the app's, not this screen's.** `Worktrees` opens and closes
 * its own because nothing else wants it; this one is also what the command
 * palette lists from, and a read the screen owned would leave the palette
 * empty on every surface but this one. So the sheet arrives as a prop.
 */
export function Manifest(props: ManifestProps) {
  // Before the branches: a hook cannot be called conditionally, and the states
  // below are branches on what was read rather than on what this holds.
  // No root file, but a workspace declares Commands: those alone are listed, and nothing of the root's.
  const workspaced = props.sheet.state === "read" && (props.sheet.sheet.workspaces ?? []).length > 0;
  const rootless = props.setUp === false && workspaced;
  const slot = useManifestRuns({ ...props, rootless });
  const { editing } = props;
  const onlySetup = props.setUp === false && props.setup !== undefined && !workspaced;
  const [dismissedVerify, setDismissedVerify] = useState<string | null>(null);
  const [verifyRefused, setVerifyRefused] = useState<string | null>(null);
  const verify = verifyPanelOf({
    sheet: props.sheet,
    now: props.now,
    dismissed: dismissedVerify,
    refused: verifyRefused,
    onVerify: () => {
      setVerifyRefused(null);
      void props.onStartVerify().then((outcome) => setVerifyRefused(outcome.ok ? null : said(outcome)));
    },
    onStopRun: (runId) => void props.onStopRun(runId),
    onDismiss: setDismissedVerify,
  });

  // A Fleet serving nothing is where a fresh install starts, not a Manifest that would not read.
  if (props.sheet.state === "failed" && servesNothing(props.sheet.outcome)) {
    return <Alert tone="neutral" title={NOTHING_SERVED.title}>{NOTHING_SERVED.next}</Alert>;
  }

  const onForm = editing.view === "form" && !props.settingUp;
  // What the view on screen is for, and the one thing about it that
  // surprises people — said once here rather than repeated on every row.
  // **Moved from the page head #1090 removed**, and it still swaps with the
  // toggle: Run's own summary is nothing here is a verdict, and the file and
  // the forms share Save's own, because both write the one file.
  const viewNote =
    props.settingUp || onlySetup
      ? null
      : editing.view === "servers"
        ? "The MCP servers a Drone dispatched against this repository is handed. Nothing else reaches one."
        : rootless || editing.view === "run"
          ? "Run one Check or Command against this checkout, as it is on disk. Nothing here is a verdict."
          : "Edit this repository's Manifest. Save writes the file to disk and stops, without staging or committing it.";
  const selected = props.settingUp || onlySetup ? "setup" : rootless ? "run" : editing.view;
  return (
    <div className="armada-screen__stack">
      {/* Above every view, so a freeze left on is seen wherever the page opens. */}
      {props.form.state !== "open" || !props.form.frozen || onlySetup || rootless ? null : (
        <Alert
          tone="neutral"
          title="This repository is frozen"
          action={
            onForm ? undefined : (
              <Button
                variant="ghost"
                size="sm"
                onClick={() => {
                  props.onSettingUp?.(false);
                  editing.onView("form");
                }}
              >
                Unfreeze on the form
              </Button>
            )
          }
        >
          Nothing lands here until it is unfrozen. New work and a running Job's next step wait, and nothing merges.
        </Alert>
      )}
      {/* Above both views, so the file stays reachable when the run sheet is
          what would not read. */}
      <Tabs
        items={[
          ...(onlySetup
            ? []
            : rootless
              ? [{ id: "run", label: "Checks and Commands" }]
              : [
                { id: "run", label: "Checks and Commands" },
                { id: "form", label: "Edit" },
                { id: "file", label: editing.named },
                { id: "servers", label: "Servers" },
              ]),
          ...(props.setup === undefined ? [] : [{ id: "setup", label: "Set up workspaces" }]),
        ]}
        value={selected}
        onChange={(id) => {
          props.onSettingUp?.(id === "setup");
          if (id !== "setup")
            editing.onView(id === "file" || id === "form" || id === "servers" ? id : "run");
        }}
      />
      {/* The view and the note that says what it is for, faded in together on a
          switch. Keyed on the selected tab, so a sheet re-reading under the
          same view does not fade. A stack of its own, so the wrapper lays its
          children out as the screen's stack did. */}
      <TabPanel tab={selected} className="armada-screen__stack" role="tabpanel">
        {viewNote === null ? null : <p className="text-fg-muted">{viewNote}</p>}
        {(props.settingUp || onlySetup) && props.setup !== undefined ? (
          props.setup
        ) : rootless ? (
          <RunPage {...slot} />
        ) : editing.view === "servers" ? (
          <ServersView {...props} />
        ) : editing.view === "file" ? (
          <FileView file={editing.file} />
        ) : editing.view === "form" ? (
          <FormView form={props.form} named={editing.named} onFile={() => editing.onView("file")} />
        ) : (
          <RunView {...props} slot={slot} verify={verify} />
        )}
      </TabPanel>
    </div>
  );
}

/**
 * Kit's servers, and what a Drone dispatched here resolves — #1275.
 *
 * **Its own read, like the always-allow list**, so a Manifest that would not
 * parse still leaves the servers readable: what a Drone is handed comes off
 * Fleet's own tables and not off `armada.yml`.
 */
function ServersView(props: ManifestProps) {
  const kit = useKit(props);
  return (
    <KitServers
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
  );
}

function RunView({
  sheet,
  slot,
  drift,
  verify,
  ...props
}: ManifestProps & {
  slot: ReturnType<typeof useManifestRuns>;
  verify: ReturnType<typeof verifyPanelOf>;
}) {
  // Drift and the always-allow list each on their own read, so a sheet that
  // would not read still leaves both standing.
  const allows = useRepositoryAllows(props);
  return (
    <>
      <DriftPanel {...driftPanelOf(drift)} />
      <SheetView sheet={sheet} slot={slot} verify={verify} allows={allows} />
    </>
  );
}

function SheetView({
  sheet,
  slot,
  verify,
  allows,
}: Pick<ManifestProps, "sheet"> & {
  slot: ReturnType<typeof useManifestRuns>;
  verify: ReturnType<typeof verifyPanelOf>;
  allows: ReturnType<typeof useRepositoryAllows>;
}) {
  if (sheet.state === "failed") {
    return (
      <Alert tone="escalated" title="This repository's Manifest could not be read">
        {said(sheet.outcome)}
      </Alert>
    );
  }
  // `none` is the frame before the app's own read has begun. It says the same
  // thing as `reading` rather than drawing three empty groups, which here
  // would claim the file declares nothing — the one answer on this page
  // nobody should be given by accident.
  if (sheet.state !== "read") {
    return <p className="text-fg-muted">Reading this repository's Manifest.</p>;
  }
  return (
    <>
      <VerifyPanel {...verify} />
      <RunPage {...slot} alwaysAllowed={allows.rows} onRemoveAlwaysAllowed={allows.onRemove} />
    </>
  );
}

function FileView({ file }: { file: ManifestEditing["file"] }) {
  if (file.state === "failed") {
    return (
      <Alert tone="escalated" title="The Manifest file could not be read">
        {file.saying}
      </Alert>
    );
  }
  if (file.state === "reading") {
    return <p className="text-fg-muted">Reading the Manifest file.</p>;
  }
  return <ManifestFile {...file.props} />;
}

function FormView({ form, named, onFile }: { form: ManifestForming; named: string; onFile: () => void }) {
  if (form.state === "failed") {
    return (
      <Alert tone="escalated" title="The Manifest file could not be read">
        {form.saying}
      </Alert>
    );
  }
  if (form.state === "unloadable") {
    // No form can be drawn from a file that does not load, and guessing at one
    // would write back what the person never saw.
    return (
      <Alert
        tone="caution"
        title="This file does not load, so there is no form to show"
        action={
          <Button variant="ghost" size="sm" onClick={onFile}>
            {`Open ${named}`}
          </Button>
        }
      >
        {`Correct ${form.path} as a file. The forms come back once it loads.`}
      </Alert>
    );
  }
  if (form.state === "reading") {
    return <p className="text-fg-muted">Reading the Manifest file.</p>;
  }
  return <ManifestForm {...form.props} />;
}
