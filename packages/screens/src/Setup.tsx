// Setup — Journey 3, *Set Up a Project*: the picker, with each workspace's proposal opening
// over it, in any order and closing back to it. Not a wizard.
//
// Drawn against the checkout this Fleet was started in; Locate (#821) is not built.
//
// # Verify lands on the sheet only for the Manifest Fleet holds
//
// `start_checkout_verify` runs the Manifest Fleet was started with, at the root. A file
// written for `apps/web` is not that one, so its sheet says so rather than offering a Verify
// that would run another file's Checks.

import { useState } from "react";
import type { CheckoutRunSheetRead, Outcome, ProposalEdit } from "@armada/protocol";
import { Alert, ProposalSheet, SetupPicker, VerifyPanel } from "@armada/components";

import { said } from "./copy";
import type { Setting } from "./setup-held";
import { pickerOf, sheetOf } from "./setup-view";
import { verifyPanelOf } from "./verify";

export type SetupProps = {
  setting: Setting;
  /** The app's one `now`. */
  now: number;
  /** What the Manifest Fleet holds declares, which is where a Verify is read from. */
  sheet: CheckoutRunSheetRead;
  onStartVerify: () => Promise<Outcome>;
  onStopRun: (runId: string) => Promise<Outcome>;
  /** The window is at `--window-floor`. */
  floor?: boolean;
};

export function Setup({ setting, now, sheet, onStartVerify, onStopRun, floor }: SetupProps) {
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
  const edit = (change: ProposalEdit) => opened !== undefined && setting.onEdit(opened.dir, change);
  const entry = (band: "checks" | "commands", name: string) =>
    band === "checks"
      ? opened?.checks.find((one) => one.name === name)
      : opened?.commands.find((one) => one.name === name);

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
          Verify runs the Manifest this Fleet was started with, at the root of the checkout. This file
          is not that one, so nothing here would run its Checks.
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
          floor={floor}
          onClose={setting.onClose}
          onEditId={(id) => edit({ edit: "id", id })}
          onEditRun={(band, name, run) => {
            const was = entry(band, name);
            if (band === "checks") {
              const requires = was !== undefined && "requires" in was ? was.requires : undefined;
              edit({ edit: "check", name, run, ...(requires === undefined ? {} : { requires }) });
            } else {
              const destructive = was !== undefined && "destructive" in was ? was.destructive : undefined;
              edit({ edit: "command", name, run, ...(destructive === undefined ? {} : { destructive }) });
            }
          }}
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
