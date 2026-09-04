// What Fleet's last read of `armada.yml` came to, as a standing condition on
// whatever surface is open.
//
// # Why this is a banner and not the status bar
//
// The design contract gives the status bar three states, five items and one
// colour, and says in as many words that anything more makes it "a second alert
// surface". A refusal has to carry a reason and a list of keys, and it has to
// offer a way to put it away; none of those fit a 32px strip, and the bar has no
// dismissal at all. The contract's own row for this shape is Banner — "above
// the surface, inside it. Persistent. The surface works beneath" — which is
// exactly true here: every Job goes on running on the last good configuration.
//
// # Why it is dismissed by hand and never on a timer
//
// A person edits a file, saves, and looks back at their editor. A toast that
// expired while they did is a refusal they never saw, and they go on editing a
// file whose values are not the ones running. The reclaimed-worktree notice
// already makes this argument in Bridge and this is the same one.

import type { ManifestReading } from "@armada/protocol";
import { worthSaying } from "@armada/protocol";

import { Alert } from "../../primitives/Alert/Alert";
import { Button } from "../../primitives/Button/Button";

export type ManifestNoticeProps = {
  /** Fleet's reading. **A reading with no news in it draws nothing** — see
   *  `worthSaying`, which is asked here rather than by every caller. */
  reading: ManifestReading;
  /** Where a person can put it away. The one ghost control `Alert` grants a
   *  standing condition, and absent where nothing may clear it. */
  onDismiss?: () => void;
};

/**
 * One reading of the Manifest, drawn.
 *
 * **A refusal and an adoption are one component and two tones**, not two
 * components. They are the same sentence about the same file — it was read, and
 * this is what came of it — and splitting them would let the two drift into
 * saying it differently.
 */
export function ManifestNotice({ reading, onDismiss }: ManifestNoticeProps) {
  // **The filter is here and not at the call site.** A save that moved nothing
  // is not news, and a surface that had to remember to ask would eventually
  // draw an empty notice — which trains a person to dismiss the one place a
  // refusal appears.
  if (!worthSaying(reading)) return null;
  const refused = reading.refused;
  return (
    <Alert
      // The one attention hue, reused rather than re-hued: a refusal is the
      // only reading here that needs somebody. An adoption is neutral because
      // nothing is wrong — the edit took.
      tone={refused === undefined ? "neutral" : "escalated"}
      title={refused === undefined ? "Manifest reloaded" : "Manifest refused"}
      action={
        onDismiss === undefined ? undefined : (
          <Button variant="ghost" size="sm" ground="sunken" onClick={onDismiss}>
            Dismiss
          </Button>
        )
      }
    >
      <div className="armada-manifest-notice">
        {refused === undefined ? null : (
          // Fleet's own sentence, carried and never rewritten. It is the only
          // part of this that can name a line and a column, because a file that
          // is not YAML at all has no keys to attribute a fault to.
          <span className="armada-manifest-notice__summary">{refused.summary}</span>
        )}

        {refused?.faults === undefined || refused.faults.length === 0 ? null : (
          <ul className="armada-manifest-notice__faults">
            {refused.faults.map((fault) => (
              <li className="armada-manifest-notice__fault" key={fault.key}>
                <span className="armada-manifest-notice__key">{fault.key}</span>
                <span className="armada-manifest-notice__fault-said">{fault.fault}</span>
              </li>
            ))}
          </ul>
        )}

        {/* The second fact, and the one that stops a person editing the file
            twice more: nothing changed, and the Jobs are still running. */}
        {refused === undefined ? null : (
          <span>The values in force are unchanged. Correct the file and save again.</span>
        )}

        {(reading.moved ?? []).map((moved) => (
          <span className="armada-manifest-notice__summary" key={moved.key}>
            {`${moved.key} is ${said(moved.after)}, was ${said(moved.before)}`}
          </span>
        ))}

        {reading.moved === undefined || reading.moved.length === 0 ? null : (
          <span>In force from the next step boundary.</span>
        )}

        {/* Named rather than swallowed. Somebody who edited `checks:` under a
            running Fleet is owed the reason nothing happened — which is the
            same silence one section over. */}
        {reading.at_restart === undefined || reading.at_restart.length === 0 ? null : (
          <span>
            {`${sections(reading.at_restart)} changed. This Fleet read that at start and will not `}
            {"read it again until it is restarted."}
          </span>
        )}
      </div>
    </Alert>
  );
}

/**
 * A value, or the word for not having one.
 *
 * **An absent key is a repository deferring to what Fleet is running with**, and
 * that reads differently from a number — so it is spelled rather than left
 * blank, which would read as a value that failed to load.
 */
function said(value: number | undefined): string {
  return value === undefined ? "unset" : String(value);
}

/** The frozen sections, listed as a person would say them. */
function sections(named: readonly string[]): string {
  if (named.length === 1) return named[0];
  return `${named.slice(0, -1).join(", ")} and ${named[named.length - 1]}`;
}
