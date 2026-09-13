// Setup — Journey 3, *Set Up a Project*: the picker, with each workspace's proposal opening
// over it, in any order and closing back to it. Not a wizard.
//
// Drawn against the repository the rail picked.
//
// # Verify lands on the sheet that wrote the file
//
// A workspace's sheet sends its directory, and Fleet runs that directory's own `armada.yml` there;
// the root's sends none. The checkout holds one Verify at a time, so each sheet draws only its own
// file's, and says so while another file's is running.

import { useState } from "react";
import type { CheckoutRunSheetRead, Outcome, ProposalEdit } from "@armada/protocol";
import { Alert, ProposalSheet, SetupPicker, VerifyPanel } from "@armada/components";

import { said } from "./copy";
import type { Setting } from "./setup-held";
import { destructiveEdit, pickerOf, requiresEdit, runEdit, sheetOf } from "./setup-view";
import { verifyPanelOf } from "./verify";

export type SetupProps = {
  setting: Setting;
  /** The app's one `now`. */
  now: number;
  /** The picked repository's run sheet, which holds the checkout's one Verify, whichever file it runs. */
  sheet: CheckoutRunSheetRead;
  /** Absent `workspace` verifies the root's file. */
  onStartVerify: (workspace?: string) => Promise<Outcome>;
  onStopRun: (runId: string) => Promise<Outcome>;
  /** Go to the Manifest's Edit tab, which edits the picked repository's root file. */
  onOpenEdit?: () => void;
  /** The window is at `--window-floor`. */
  floor?: boolean;
};

export function Setup({ setting, now, sheet, onStartVerify, onStopRun, onOpenEdit, floor }: SetupProps) {
  const [dismissed, setDismissed] = useState<string | null>(null);
  // Keyed by workspace, so a refusal on one sheet is not read on another.
  const [verifyRefused, setVerifyRefused] = useState<{ dir: string; said: string } | null>(null);
  const { held } = setting;

  if (held.state === "failed") {
    return (
      <Alert tone="escalated" title="This checkout's workspaces could not be read">
        {held.saying}
      </Alert>
    );
  }
  if (held.state === "reading") {
    return <p className="text-fg-muted">Reading this checkout's workspaces.</p>;
  }

  const opened = held.proposals.proposals.find((one) => one.dir === held.open);
  const edit = (change: ProposalEdit | null) => opened !== undefined && change !== null && setting.onEdit(opened.dir, change);

  let verify = null;
  if (opened?.written !== undefined) {
    const { dir } = opened;
    const workspace = dir === "." ? undefined : dir;
    verify = (
      <VerifyPanel
        {...verifyPanelOf({
          sheet,
          now,
          dismissed,
          refused: verifyRefused?.dir === dir ? verifyRefused.said : null,
          ...(workspace === undefined ? {} : { workspace }),
          onVerify: () => {
            setVerifyRefused(null);
            void onStartVerify(workspace).then((outcome) => setVerifyRefused(outcome.ok ? null : { dir, said: said(outcome) }));
          },
          onStopRun: (runId) => void onStopRun(runId),
          onDismiss: setDismissed,
        })}
      />
    );
  }

  return (
    <div className="armada-screen__stack">
      <SetupPicker {...pickerOf(held)} onTick={setting.onTick} onOpen={setting.onOpen} />
      {opened === undefined ? null : (
        <ProposalSheet
          open
          {...sheetOf(held, opened)}
          busy={held.busy === opened.dir}
          verify={verify}
          // Only the root's file is the one the Edit tab edits.
          {...(opened.dir === "." && onOpenEdit !== undefined ? { onEditManifest: onOpenEdit } : {})}
          floor={floor}
          onClose={setting.onClose}
          onEditId={(id) => edit({ edit: "id", id })}
          onEditRun={(band, name, run) => edit(runEdit(opened, band, name, run))}
          onRequires={(name, requires) => edit(requiresEdit(opened, name, requires))}
          onDestructive={(name, destructive) => edit(destructiveEdit(opened, name, destructive))}
          onMove={(_band, name) => edit({ edit: "move", name })}
          onRemove={(band, name) => edit({ edit: "remove", band, name })}
          onAdd={(band, name, run) => edit(band === "checks" ? { edit: "check", name, run } : { edit: "command", name, run })}
          onAddPort={(port) =>
            edit({ edit: "port", name: port.name, container: Number(port.container), ...(port.env === "" ? {} : { env: port.env }) })
          }
          onPolicy={(key, value) => edit({ edit: "policy", key, value })}
          onWrite={() => setting.onWrite(opened.dir)}
        />
      )}
    </div>
  );
}
