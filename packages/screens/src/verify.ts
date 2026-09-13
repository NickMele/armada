// The Manifest surface's two panels for Journey 9's *Verify* — drift, read on
// opening, and Verify, pressed — built from what Fleet serves.
//
// **Two readings, kept apart.** Drift comes off `GET /manifest/drift` and says
// whether the file is still true; Verify comes off the run sheet's `verify` and
// says whether it still works. Neither is folded into the other here, for the
// journey's reason: one panel with two verdict groups reads as an audit.

import type { DriftPanelProps, DriftPanelRow, VerifyPanelProps, VerifyPanelStep } from "@armada/components";
import type {
  CheckoutRunRecord,
  CheckoutRunSheetRead,
  CheckoutRunUnderway,
  CheckoutVerify,
  Declaration,
  ManifestDriftRead,
  VerifyStep,
} from "@armada/protocol";
import { said } from "./copy";
import { span } from "./duration";

/** Drift as the panel draws it. A read that has not answered says so. */
export function driftPanelOf(read: ManifestDriftRead): DriftPanelProps {
  switch (read.state) {
    case "none":
    case "reading":
      return { note: "Reading whether this checkout still has what armada.yml names." };
    case "failed":
      return { note: `Drift could not be read. ${said(read.outcome)}` };
    case "read":
      return { rows: read.drift.declarations.map(driftRowOf) };
  }
}

function driftRowOf(line: Declaration, at: number): DriftPanelRow {
  const where = `${line.section}.${line.name}.${line.key}`;
  return {
    id: `${at}:${where}`,
    where,
    run: line.run,
    verdict: line.drift.verdict === "gone" ? "gone" : "current",
    ...(line.drift.verdict === "gone" ? { missing: line.drift.missing } : {}),
    unfollowed: line.unfollowed,
  };
}

/** What the Verify panel needs of the surface. */
export type VerifyInputs = {
  sheet: CheckoutRunSheetRead;
  now: number;
  /** The Verify a person put away, by id. The next one shows again. */
  dismissed: string | null;
  /** Why the last press was refused, or `null`. */
  refused: string | null;
  /** The workspace whose file this panel verifies. Absent, or `.`, is the root's. */
  workspace?: string;
  onVerify: () => void;
  onStopRun: (runId: string) => void;
  onDismiss: (verifyId: string) => void;
};

/**
 * The Verify panel's props. **Verify is offered only where nothing is out** —
 * a run a person started holds the checkout's one slot, and a Verify already
 * underway holds it between steps too.
 */
export function verifyPanelOf(inputs: VerifyInputs): VerifyPanelProps {
  const data = inputs.sheet.state === "read" ? inputs.sheet.sheet : undefined;
  // The checkout holds one Verify: another file's is not drawn here, and holds the slot until it ends.
  const held = data?.verify;
  const verify = held !== undefined && fileOf(held.workspace) === fileOf(inputs.workspace) ? held : undefined;
  const other = held !== undefined && verify === undefined && held.ended_at === undefined ? held : undefined;
  const underway = verify !== undefined && verify.ended_at === undefined;
  const shown = verify !== undefined && verify.id !== inputs.dismissed ? verify : undefined;
  const out = verify?.steps.find((step) => step.state === "running");

  let offer: Pick<VerifyPanelProps, "onVerify" | "unavailable"> = { onVerify: inputs.onVerify };
  if (data === undefined) offer = { unavailable: "Verify can start once this Manifest has been read." };
  else if (underway) offer = {};
  else if (other !== undefined)
    offer = { unavailable: `Verify is running ${fileOf(other.workspace)} in this checkout. This file can be verified once it ends.` };
  else if (data.running !== undefined)
    offer = { unavailable: `\`${data.running.name}\` is running in this checkout. Verify can start once it ends.` };

  return {
    ...offer,
    ...(shown === undefined
      ? {}
      : { steps: shown.steps.map((step, at) => verifyStepOf(step, at, data?.running, inputs.now)) }),
    ...(shown?.ended_at === undefined
      ? {}
      : { ended: endedOf(shown), onDismiss: () => inputs.onDismiss(shown.id) }),
    ...(inputs.refused === null ? {} : { refused: inputs.refused }),
    ...(out?.state === "running" ? { onStop: () => inputs.onStopRun(out.run_id) } : {}),
  };
}

/** The file a Verify runs, as the repository names it. */
export function fileOf(workspace: string | undefined): string {
  return workspace === undefined || workspace === "" || workspace === "." ? "armada.yml" : `${workspace}/armada.yml`;
}

function verifyStepOf(
  step: VerifyStep,
  at: number,
  running: CheckoutRunUnderway | undefined,
  now: number,
): VerifyPanelStep {
  const drawn = {
    id: `${at}:${step.name}`,
    group: step.group === "setup" ? "Setup" : step.group === "checks" ? "Checks" : step.group,
    name: step.name,
    run: step.run,
  };
  switch (step.state) {
    case "waiting":
      return { ...drawn, state: { kind: "waiting" } };
    case "running": {
      const since = running?.id === step.run_id ? span(running.started_at, now) : undefined;
      return { ...drawn, state: { kind: "running", elapsed: since ?? "just started" } };
    }
    case "ran":
      return {
        ...drawn,
        state: {
          kind: "ran",
          result: resultOf(step.record),
          duration: `${(step.record.duration_ms / 1000).toFixed(1)}s`,
        },
      };
    case "not_run":
      return { ...drawn, state: { kind: "not_run", why: step.why } };
  }
}

/** The exit code beside the one expected, or how it ended. Unhued. */
function resultOf(record: CheckoutRunRecord): string {
  return record.exit_code === undefined
    ? record.ended
    : `exit ${record.exit_code} (expects ${record.expect_exit_code})`;
}

/**
 * One line of counts once a Verify has ended. **Counts, never a verdict**: a
 * code other than the one expected is said as that, not as a failure.
 */
export function endedOf(verify: CheckoutVerify): string {
  let ran = 0;
  let otherwise = 0;
  for (const step of verify.steps) {
    if (step.state !== "ran") continue;
    ran += 1;
    if (step.record.exit_code !== step.record.expect_exit_code) otherwise += 1;
  }
  const notRun = verify.steps.filter((step) => step.state === "not_run").length;
  return [
    `Ran ${ran} of ${verify.steps.length}.`,
    otherwise === 0
      ? null
      : `${otherwise} ended with a code other than the one ${otherwise === 1 ? "it expects" : "they expect"}.`,
    notRun === 0 ? null : `${notRun} did not run.`,
  ]
    .filter((part) => part !== null)
    .join(" ");
}
