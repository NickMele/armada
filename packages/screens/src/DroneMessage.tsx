// The message box fixed under an activity log, wired to the redirect — #1154.
//
// **The control and not the dialog.** `Redirect.tsx` is the button and the
// confirmation for the moment the log is not open; this is the same act,
// `onRedirect`, for the moment it is — one text field, no confirmation, because
// the box is not reached by accident the way a key binding can be.
//
// **One mount, read twice.** The inline chapter and the Activity log sheet each
// place this at their own foot rather than sharing a DOM node, so the state a
// person is mid-typing in one does not leak into the other — the same reason
// `RedirectControl` keeps its own instruction rather than lifting it.

import { useState } from "react";
import { DroneMessageBox } from "@armada/components";

import type { JobDetail as JobWhole, JobSummary } from "@armada/protocol";
import { renderFor } from "./render";
import { steeringOf } from "./steering";

/**
 * Why the box is off. **One sentence for three states** — queued, between
 * steps, finished — because `steeringOf` does not say which of them a job is
 * in, only that no drone is there to read a message.
 */
const NO_DRONE_TO_MESSAGE = "No drone is on this step, so there's nothing to send this to.";

/**
 * Sends a redirect from wherever a person is reading the log.
 *
 * **Reads `steeringOf`, and only it** — the same reading `StepActs.tsx` takes
 * before offering the button, so the two can never disagree about whether a
 * drone can hear. A job that has stopped is `recovery.ts`'s half of the same
 * act, offered nowhere here: the button and its dialog stay the only way to
 * redirect a drone that is holding rather than working.
 */
export function DroneMessageControl({
  job,
  whole,
  onRedirect,
}: {
  job: JobSummary;
  whole: JobWhole | null;
  onRedirect: (jobId: string, instruction: string) => void;
}) {
  const [value, setValue] = useState("");
  const steering = renderFor(job) === "working" ? steeringOf(job, whole) : undefined;
  const disabled = steering?.act !== "redirect";

  function send(): void {
    const sent = value;
    setValue("");
    onRedirect(job.id, sent);
  }

  return (
    <DroneMessageBox
      value={value}
      onChange={setValue}
      onSend={send}
      disabled={disabled}
      {...(disabled ? { disabledReason: NO_DRONE_TO_MESSAGE } : {})}
      {...(steering?.sent === undefined ? {} : { waiting: steering.sent })}
    />
  );
}
