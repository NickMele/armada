// Setup — Journey 3, *Set Up a Project*: the picker, with each workspace's proposal opening
// over it, in any order and closing back to it. Not a wizard.
//
// Drawn against the repository the rail picked.
//
// # Verify lands on the sheet only for the root's file
//
// `start_checkout_verify` runs the picked repository's Manifest, at its root. A file written
// for `apps/web` is not that one, so its sheet says so rather than offering a Verify that
// would run another file's Checks.

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
  /** What the picked repository's Manifest declares, which is where a Verify is read from. */
  sheet: CheckoutRunSheetRead;
  onStartVerify: () => Promise<Outcome>;
  onStopRun: (runId: string) => Promise<Outcome>;
  /** Go to the Manifest's Edit tab, which edits the picked repository's root file. */
  onOpenEdit?: () => void;
  /** The window is at `--window-floor`. */
  floor?: boolean;
};

export function Setup({ setting, now, sheet, onStartVerify, onStopRun, onOpenEdit, floor }: SetupProps) {
  const [dismissed, setDismissed] = useState<string | null>(null);
  const [verifyRefused, setVerifyRefused] = useState<string | null>(null);
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
    verify =
      opened.dir === "." ? (
        <VerifyPanel
          {...verifyPanelOf({
            sheet,
            now,
            dismissed,
            refused: verifyRefused,
            onVerify: () => {
              setVerifyRefused(null);
              void onStartVerify().then((outcome) => setVerifyRefused(outcome.ok ? null : said(outcome)));
            },
            onStopRun: (runId) => void onStopRun(runId),
            onDismiss: setDismissed,
          })}
        />
      ) : (
        <p className="text-fg-muted">
          Verify runs this repository's Manifest, at the root of the checkout. This file is not that
          one, so nothing here would run its Checks.
        </p>
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
