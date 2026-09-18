// What a job that was replaced says about itself, under the header. A killed
// and redispatched job used to be a dead end: it said killed, and nothing said
// the work had carried on somewhere else. #1439.

import { Alert, Button } from "@armada/components";
import type { ReplacedBy } from "@armada/protocol";

/**
 * The standing condition on a job a redispatch replaced.
 *
 * **Neutral, and no glyph.** Nothing is broken — a redispatch is Armada
 * working — and the header's badge already carries the status in its hue. The
 * error treatment's glyphs are spoken for and the contract mints no generic
 * one.
 *
 * **The handle is on the control and nowhere else**: one value names the
 * replacement and reaches it, rather than a sentence repeating the button.
 *
 * `undefined` draws nothing — every job nothing replaced, and every job whose
 * replacement has been forgotten.
 */
export function replacedCallout(
  replaced: ReplacedBy | undefined,
  onOpenJob: ((jobId: string) => void) | undefined,
): React.ReactNode {
  if (replaced === undefined) return undefined;
  return (
    <Alert
      tone="neutral"
      title="This job was redispatched"
      action={
        onOpenJob === undefined ? undefined : (
          <Button variant="ghost" size="sm" onClick={() => onOpenJob(replaced.job_id)}>
            {`Open ${replaced.handle}`}
          </Button>
        )
      }
    >
      The work carried on as a new job. Nothing more happens on this one.
    </Alert>
  );
}
