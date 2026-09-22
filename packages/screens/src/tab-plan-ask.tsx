// Asking the plan's Drone for a different split, and what is said before the
// ask goes out. `#1552`.
//
// **Its own file, for `Redirect.tsx`'s reason.** Which acts a destination
// offers is one subject, and what a single act asks a person for before it
// sends is another — this is the second, and `tab-plan.tsx` is the first.
//
// **The ask reaches the Drone as a redirect**, which is the one command that
// turns into a session already open. Nothing new goes on the wire: a plan
// revision is a person saying something to the Drone that wrote the plan, and
// `redirect` is what that has always been.

import { useState } from "react";
import { Dialog, Textarea } from "@armada/components";

import type { GroupView } from "./draft/group";
import type { PlanAskKind } from "./draft/revision";
import { reorderWarning } from "./tab-plan-read";

/** A group ask a person has opened and not yet sent. */
export type PlanAskInFlight = { group: string; ask: PlanAskKind };

/** Which group the ask is on, or `undefined` where the plan has no such group. */
function groupOf(groups: readonly GroupView[], id: string): GroupView | undefined {
  return groups.find((one) => one.id === id);
}

/** The group this ask would put the moving one on the other side of. */
function partnerOf(groups: readonly GroupView[], id: string, ask: PlanAskKind): GroupView | undefined {
  const at = groups.findIndex((one) => one.id === id);
  if (at < 0) return undefined;
  if (ask === "move_up") return groups[at - 1];
  if (ask === "move_down") return groups[at + 1];
  return undefined;
}

/**
 * What is being asked, as the dialog's own title.
 *
 * **It names the new order rather than the control.** `Move group 3 up?`
 * restates the button; `Run group 3 before group 2?` is the plan a person is
 * asking for, which is the thing the Drone will answer.
 */
export function askTitle(groups: readonly GroupView[], id: string, ask: PlanAskKind): string {
  const group = groupOf(groups, id);
  if (group === undefined) return "Ask the Drone to change the plan?";
  const partner = partnerOf(groups, id, ask);
  if (ask === "move_up" && partner !== undefined) {
    return `Ask the Drone to run group ${group.ordinal} before group ${partner.ordinal}?`;
  }
  if (ask === "move_down" && partner !== undefined) {
    return `Ask the Drone to run group ${group.ordinal} after group ${partner.ordinal}?`;
  }
  return `Ask the Drone to drop group ${group.ordinal}?`;
}

/** Which tasks go with the group, so a remove names what it takes with it. */
export function askHolds(groups: readonly GroupView[], id: string): string | undefined {
  const group = groupOf(groups, id);
  if (group === undefined || group.tasks.length === 0) return undefined;
  const ids = group.tasks.map((task) => task.id).join(", ");
  return `Group ${group.ordinal} holds ${ids}.`;
}

/**
 * What the Drone is told, as one instruction.
 *
 * **The second paragraph is the whole of `#1552`.** Without it the Drone
 * reads a reorder as work to do; with it, refusing is an answer rather than a
 * failure to comply. The mechanical warning rides along because the Drone is
 * the one that knows whether it matters.
 */
export function askInstruction(
  groups: readonly GroupView[],
  id: string,
  ask: PlanAskKind,
  note: string,
): string {
  const group = groupOf(groups, id);
  const partner = partnerOf(groups, id, ask);
  const ordinal = group?.ordinal ?? 0;
  const asked =
    ask === "move_up" && partner !== undefined
      ? `Run group ${ordinal} before group ${partner.ordinal}.`
      : ask === "move_down" && partner !== undefined
        ? `Run group ${ordinal} after group ${partner.ordinal}.`
        : `Drop group ${ordinal}${askHolds(groups, id) === undefined ? "" : ` and the tasks in it`}.`;
  const lines = [
    `A change to the plan you recorded: ${asked}`,
    "This is a request about the split, not about the code. Where the order you chose has a reason the plan does not give, refuse it and say what that reason is.",
  ];
  const warning = reorderWarning(groups, id, ask);
  if (warning !== undefined) lines.push(warning);
  if (note !== "") lines.push(note);
  return lines.join("\n\n");
}

/** The rewrite ask, on the same terms, addressed to one task. */
export function rewriteInstruction(taskId: string, note: string): string {
  return [
    `A change to the plan you recorded, on ${taskId}: ${note}`,
    "This is a request about the split, not about the code. Where the task is written the way it is for a reason the plan does not give, refuse it and say what that reason is.",
  ].join("\n\n");
}

/**
 * The dialog a group ask is confirmed in.
 *
 * **The dialog is the confirmation**, `RedirectControl`'s rule: sending is the
 * one thing its confirm does, and closing it any other way sends nothing. It
 * exists rather than the press going straight out because the mechanical
 * warning has to be read before the ask, not after it.
 */
export function PlanAskDialog({
  groups,
  inFlight,
  onCancel,
  onSend,
}: {
  groups: readonly GroupView[];
  /** The ask a person opened, or nothing where no dialog is open. */
  inFlight: PlanAskInFlight | null;
  onCancel: () => void;
  onSend: (instruction: string) => void;
}) {
  const [note, setNote] = useState("");

  function close() {
    setNote("");
    onCancel();
  }

  // Unmounted rather than closed, so the next ask opens on an empty field
  // rather than on the last one's reason.
  if (inFlight === null) return null;

  const warning = reorderWarning(groups, inFlight.group, inFlight.ask);
  const holds = askHolds(groups, inFlight.group);
  return (
    <Dialog
      open
      tone="neutral"
      title={askTitle(groups, inFlight.group, inFlight.ask)}
      confirmLabel="Ask"
      onCancel={close}
      onConfirm={() => {
        onSend(askInstruction(groups, inFlight.group, inFlight.ask, note.trim()));
        setNote("");
      }}
      field={
        <Textarea
          label="Why, in your words"
          rows={3}
          value={note}
          placeholder="The rows are what the tests hold, so they have to exist before the panel does"
          onChange={(event) => setNote(event.target.value)}
        />
      }
    >
      <p>
        The plan is a record the Drone wrote. This asks it for a different split. It may take the
        change, and it may refuse and say why.
      </p>
      {holds === undefined ? null : <p>{holds}</p>}
      {/* The one mechanical catch there is. A warning rather than a refusal:
          the reorder is legal, and what it costs is a file written in the
          other order. */}
      {warning === undefined ? null : (
        <p className="armada-plan-tab__ask-warning" role="note">
          {warning}
        </p>
      )}
    </Dialog>
  );
}
