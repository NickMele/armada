// The Manifest surface — Journey 9's *Running one*, with no Job in existence.
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
// this `⌘4`. Until now it reached nothing.
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
// # What is not built here, and is this surface's by the journey
//
// Editing and Verify. Journey 9 puts both on this surface, and neither has a
// wire: `armada.yml` is read-only over the connection today and nothing serves
// drift. They are absent rather than stubbed — a form that could not save and
// a Verify button that could not run would be two labelled blanks.

import { Alert } from "@armada/components";
import { RunPage } from "@armada/components";

import { said } from "./copy";
import { useManifestRuns, type ManifestSlice } from "./checkout-runs";

export type ManifestProps = ManifestSlice & {
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
};

/**
 * Everything this repository's Manifest declares, and one press that runs one
 * of them in the checkout as it is on disk.
 *
 * **The read is the app's, not this screen's.** `Worktrees` opens and closes
 * its own because nothing else wants it; this one is also what the command
 * palette lists from, and a read the screen owned would leave the palette
 * empty on every surface but this one. So the sheet arrives as a prop.
 */
export function Manifest(props: ManifestProps) {
  // Before the branches: a hook cannot be called conditionally, and the states
  // below are branches on what was read rather than on what this holds.
  const slot = useManifestRuns(props);

  if (props.sheet.state === "failed") {
    return (
      <Alert tone="escalated" title="This repository's Manifest could not be read">
        {said(props.sheet.outcome)}
      </Alert>
    );
  }
  // `none` is the frame before the app's own read has begun. It says the same
  // thing as `reading` rather than drawing three empty groups, which here
  // would claim the file declares nothing — the one answer on this page
  // nobody should be given by accident.
  if (props.sheet.state !== "read") {
    return <p className="text-fg-muted">Reading this repository's Manifest.</p>;
  }

  return (
    <div className="armada-screen__stack">
      <RunPage {...slot} />
    </div>
  );
}
