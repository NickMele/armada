import type { Meta, StoryObj } from "@storybook/react-vite";
import { File } from "lucide-react";
import { expect } from "storybook/test";

import { Button } from "../../primitives/Button/Button";
import { FailureNotice } from "./FailureNotice";

/**
 * The failures Bridge has to tell apart, drawn side by side so nobody has to
 * break the app to see one.
 *
 * They look identical today and demand different things: Fleet unreachable
 * needs to say *which* of the four answers the runtime file gave, a renderer
 * throw needs the component and the rest of the app still usable, and a Job
 * that will not load is one bad row rather than a broken board.
 *
 * **Every one of them carries a code, and only one of them was given one.**
 * The chip is what says *error rather than status* on a surface where an error
 * and a failed Job are the same red, so the code is required — and the four
 * that never crossed the wire mint their own in the `bridge.` namespace. A
 * story here is the visible half of that: `ARM-0412` came off a `WireError`,
 * and every `bridge.*` value below was minted by the process that raised it.
 *
 * Every story points at a log as a machine value — mono, copy on click, no
 * `copy` glyph, a toast confirms. A run id appears on the one failure that
 * really has one, labelled as what it is: Fleet mints it once per process, so
 * it names a session rather than a failure, and the rest show none rather than
 * a labelled blank. **A minted code and no run id is not an inconsistency** —
 * a code names a kind of failure Bridge knows, and a run id would have named a
 * Fleet process it never reached.
 */
const meta: Meta<typeof FailureNotice> = {
  title: "Compositions/Failure notice",
  component: FailureNotice,
};
export default meta;

type Story = StoryObj<typeof FailureNotice>;

/** Fixtures. */
const AUDIT = "/Users/user/Library/Application Support/Armada/audit.jsonl";
const RUNTIME_FILE = "/Users/user/Library/Application Support/Armada/fleet.json";

/**
 * The one real run id on this surface. Measured against a live daemon: Fleet
 * mints it once per process and every answer clones it, so four refusals in one
 * session quote this same value. The row says "Fleet run" for that reason.
 */
const FLEET_RUN = "01M0ZTNSVD0004FY52G3PP82SJ";

/**
 * The controls a failure carries. Ghost, because none of them is a decision
 * Armada participates in — a redraw and a clipboard write.
 *
 * Reload appears wherever redrawing re-runs the thing that failed: a
 * reconnect, a re-render, a re-read of the board. It is absent on a refusal,
 * where the command was answered and reloading answers nothing.
 */
function Acts({ reload = true, dismiss = false }: { reload?: boolean; dismiss?: boolean }) {
  return (
    <>
      {reload ? (
        <Button variant="ghost" size="sm" ground="sunken">
          Reload Bridge
        </Button>
      ) : null}
      <Button variant="ghost" size="sm" ground="sunken">
        Copy report
      </Button>
      {dismiss ? (
        <Button variant="ghost" size="sm" ground="sunken">
          Dismiss
        </Button>
      ) : null}
    </>
  );
}

/**
 * **Fleet unreachable, answer one of four: no runtime file.** Nothing is
 * running and nothing wrote a file, so there is no pid to check and no port to
 * open. Retrying is pointless until somebody starts Fleet, which Bridge cannot
 * do — so the sentence says who does.
 */
export const FleetIsNotRunningNoFile: Story = {
  args: {
    headline: "Fleet is not running",
    code: "bridge.fleet.not_running",
    next: "Start Fleet. Bridge reconnects on its own.",
    detailsLabel: "What the runtime file answered",
    details: [
      { label: "Answer", value: "no_runtime_file" },
      { label: "Runtime file", value: RUNTIME_FILE },
    ],
    values: [
      { icon: File, iconLabel: "Log", value: AUDIT, copyValue: AUDIT },
    ],
    note: "Bridge rereads the file every 2 seconds and connects when one appears. Nothing reached Fleet, so there is no run id to quote.",
    actions: <Acts />,
  },
};

/**
 * **Answer two: a pid nothing holds.** The file is there and Fleet exited
 * without cleaning it up. The distinction from answer one is what a person
 * does next about the stale file, so it is not folded into one message.
 */
export const FleetIsNotRunningDeadPid: Story = {
  args: {
    headline: "Fleet is not running",
    code: "bridge.fleet.not_running",
    next: "Start Fleet. Bridge reconnects on its own.",
    detailsLabel: "What the runtime file answered",
    details: [
      { label: "Answer", value: "pid_dead" },
      { label: "Pid", value: "48221" },
      { label: "Runtime file", value: RUNTIME_FILE },
    ],
    values: [
      { icon: File, iconLabel: "Log", value: AUDIT, copyValue: AUDIT },
    ],
    note: "Fleet exited without cleaning up. The file names a pid nothing holds.",
    actions: <Acts />,
  },
};

/**
 * **Answer three: a pid something else now holds.** The row a bare liveness
 * check gets wrong, and the one that must never become a socket — the port in
 * this file belongs to an unrelated program.
 */
export const FleetIsNotRunningPidReused: Story = {
  args: {
    headline: "Fleet is not running",
    code: "bridge.fleet.not_running",
    next: "Start Fleet. Bridge reconnects on its own.",
    detailsLabel: "What the runtime file answered",
    details: [
      { label: "Answer", value: "pid_held_by_another" },
      { label: "Pid", value: "48221" },
      { label: "File wrote", value: "Sat Aug 23 09:14:02 2026" },
      { label: "Holder started", value: "Sun Aug 24 18:02:55 2026" },
      { label: "Runtime file", value: RUNTIME_FILE },
    ],
    values: [
      { icon: File, iconLabel: "Log", value: AUDIT, copyValue: AUDIT },
    ],
    note: "Bridge did not open a socket. The port in this file is not Fleet's.",
    actions: <Acts />,
  },
};

/**
 * **Answer four: running, and silent.** The pid checks out and the socket does
 * not answer. **This is the one worth retrying**, and it is the one where
 * restarting Fleet is the wrong fix — so the sentence does not say to.
 */
export const FleetIsUnreachable: Story = {
  args: {
    headline: "Fleet unreachable",
    code: "bridge.fleet.unreachable",
    next: "Fleet is up and not answering. What is on the board is not live.",
    detailsLabel: "What the connection answered",
    details: [
      { label: "Pid", value: "48221" },
      { label: "Port", value: "7773" },
      { label: "Silent for", value: "1m" },
      { label: "Detail", value: "the connection closed" },
      { label: "Last read", value: "1m ago" },
    ],
    values: [
      { icon: File, iconLabel: "Log", value: AUDIT, copyValue: AUDIT },
    ],
    note: "Bridge is retrying every 2 seconds. Jobs keep progressing either way.",
    actions: <Acts />,
  },
};

/**
 * **The renderer threw.** The failure this whole notice exists for: a React
 * error with no boundary blanks the window, and a title bar over an empty
 * window says nothing about which of the three happened.
 *
 * The headline names the region in the app's voice; the component the stack
 * names is folded away with the message and the stack itself. Reloading is
 * safe to state flatly, because Bridge and Fleet have independent lifetimes.
 *
 * **No run id row, and no labelled blank where one would go.** This never
 * reached Fleet, so there is no run to name; the note says what identifies it
 * instead.
 *
 * **One code for every boundary, not one per region.** The region names what
 * stopped drawing rather than what went wrong, so it travels as a field where
 * it can be joined to the component — which is the objection to the version of
 * this that drew the region in the chip.
 */
export const TheRendererThrew: Story = {
  args: {
    headline: "Bridge could not draw the job list",
    code: "bridge.render.boundary",
    next: "Reload Bridge. Fleet keeps running and jobs keep progressing.",
    detailsLabel: "What threw",
    details: [
      { label: "Component", value: "JobRowStacked" },
      { label: "Message", value: "Cannot read properties of undefined (reading 'steps')" },
      {
        label: "Where",
        value: "    at JobRowStacked\n    at Row\n    at Jobs\n    at Boundary\n    at App",
      },
      {
        label: "Stack",
        value:
          "TypeError: Cannot read properties of undefined (reading 'steps')\n" +
          "    at Row (Jobs.tsx:214:29)\n" +
          "    at renderWithHooks (react-dom-client.js:5529:24)",
      },
    ],
    values: [
      { icon: File, iconLabel: "Log", value: AUDIT, copyValue: AUDIT },
    ],
    note: "The rest of the window is still usable. Only this region stopped drawing. This never reached Fleet, so there is no run id: the component and the log identify it.",
    actions: <Acts />,
  },
  /**
   * **Folded, not dropped — which is the whole treatment.** A stack on screen
   * is the generic error page this component exists instead of; a stack that is
   * not in the document at all is a failure nobody can report. The assertions
   * are on the alert's own text and on what a person can see, so the fold can
   * be rebuilt out of anything that keeps that true.
   */
  play: async ({ canvas, userEvent }) => {
    const notice = canvas.getByRole("alert");
    await expect(notice).toHaveTextContent("Bridge could not draw the job list");
    await expect(notice).toHaveTextContent("Reload Bridge.");

    // In the document, and not on screen, until it is asked for.
    await expect(canvas.getByText("JobRowStacked")).not.toBeVisible();
    await userEvent.click(canvas.getByText("What threw"));
    await expect(canvas.getByText("JobRowStacked")).toBeVisible();

    // The one thing a person can do about it is a control, in both states.
    await expect(canvas.getByRole("button", { name: "Reload Bridge" })).toBeVisible();
  },
};

/**
 * **A Job cannot be read.** `LoadAllError` returns what loaded beside what
 * failed, so one bad row is not a broken app — and hiding it is worse than
 * drawing it broken. The board above still lists every Job that loaded.
 *
 * Two things the wire does not carry, said out loud rather than guessed: the
 * `run_id` of the Fleet line that refused the row, and which repository the
 * Job's log sits in. The path is repo-relative because that is all Bridge
 * knows.
 */
export const AJobCannotBeRead: Story = {
  args: {
    headline: "Job 01K2Y0X6R4B7QW9V3N5T8CJ1MF did not load",
    code: "bridge.job.unreadable",
    next: "Every other job on the board is unaffected. Read the fault, or read the job's log.",
    detailsLabel: "What the store refused",
    details: [
      { label: "Job", value: "01K2Y0X6R4B7QW9V3N5T8CJ1MF" },
      {
        label: "Fault",
        value: "row 14: status `awaiting_attestation` has no attestation, which the model requires",
      },
    ],
    values: [
      {
        icon: File,
        iconLabel: "Log",
        value: ".armada/logs/01K2Y0X6R4B7QW9V3N5T8CJ1MF.jsonl",
        copyValue: ".armada/logs/01K2Y0X6R4B7QW9V3N5T8CJ1MF.jsonl",
      },
      { icon: File, iconLabel: "Log", value: AUDIT, copyValue: AUDIT, separated: true },
    ],
    note: "The log path is relative to the job's repository. Fleet does not send which one, and it does not send a run id for the read that refused this row.",
    actions: <Acts />,
  },
};

/**
 * **A command Fleet refused.** The only failure on this surface that carries a
 * run id, because it is the only one minted on the other side of the
 * connection. The row is labelled "Fleet run" and the note says what that is:
 * Fleet mints it once per process, so every refusal in one session quotes it.
 *
 * **The one code on this surface that Bridge did not mint.** It is opaque —
 * looked up, never parsed — and it carries no `bridge.` prefix, which is how a
 * reader tells at a glance that Fleet raised it and a manifest holds what it
 * means. `fields` and `chain` travel on every `WireError` and are folded away
 * whole rather than summarised — a refusal's `message` names one problem even
 * where several exist, which was measured against a live daemon.
 */
export const FleetRefusedTheCommand: Story = {
  args: {
    headline: "Manifest 01K1M8Z5V2 is not one Fleet holds",
    code: "ARM-0412",
    next: "Nothing was sent. Change what the command names, or read the log.",
    detailsLabel: "What Fleet refused",
    details: [
      { label: "Code", value: "ARM-0412" },
      { label: "Message", value: "Manifest 01K1M8Z5V2 is not one Fleet holds" },
      { label: "manifest_id", value: "01K1M8Z5V2QW7H3N9TB4XC6RFD" },
      { label: "held", value: "3" },
      {
        label: "Chain",
        value: "propose_job\nresolve_manifest\nManifestNotHeld",
      },
    ],
    values: [
      { icon: File, iconLabel: "Log", value: AUDIT, copyValue: AUDIT },
      { value: FLEET_RUN, copyValue: FLEET_RUN, meta: "Fleet run" },
    ],
    note: "The run names Fleet's process for this session, not this one failure. It is what joins this to Fleet's log lines.",
    actions: <Acts reload={false} dismiss />,
  },
};

/**
 * **The smallest legal notice.** No details to fold, no machine value to copy
 * — a failure that still names itself and still says what to do, because a
 * failure with nothing to do is drawn as the dead end it is rather than left
 * blank.
 *
 * The code is not among what falls away. A notice with a headline and nothing
 * else still has to say *error rather than status*, and the chip is one of the
 * two channels that do it.
 */
export const NothingButTheSentence: Story = {
  args: {
    headline: "Bridge could not reach the main process",
    code: "bridge.uncaught.rejection",
    next: "Reload Bridge. If it happens again, quit and reopen.",
  },
};
