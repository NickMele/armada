import { Button } from "../../primitives/Button/Button";
import { KbdChord } from "../../primitives/Kbd/Kbd";

/**
 * The bar Bridge owns above another repository's running web app —
 * `docs/practices/capture-window.md`, *What a person sees*; `#1294`.
 *
 * **A person has to tell at a glance that what is below this is not Armada.**
 * So it names the Run, the address in full and the Studio a Note will land on,
 * and it is drawn outside the page rather than in it — a bar composited above
 * is a bar the page cannot draw over.
 *
 * **There is no address field, no Back and no Forward.** This is not a browser:
 * the window is pinned to one origin and Reload reloads that one.
 */

/** An address this window refused, as the bar says it. */
export type CaptureBarRefusal = {
  address: string;
  /** What moved: a navigation, a redirect, a popup or a download. */
  said: string;
  /** Whether the system browser would take it. */
  offerable: boolean;
};

export type CaptureBarProps = {
  /** What the Manifest calls the server this window is on. */
  run: string;
  /** Scheme, host and port, drawn in full and never abbreviated. */
  address: string;
  /** The Studio a Note lands on, by name. */
  studio: string;
  /** Whether the Run is still serving. Capture ends with it. */
  serving: boolean;
  /** Whether capture is armed. */
  armed: boolean;
  /** How many subframe navigations were refused. Counted, never named. */
  framesRefused: number;
  refused?: CaptureBarRefusal;
  onArm: (on: boolean) => void;
  onReload: () => void;
  /** Hand the refused address to the browser this person already uses. */
  onFollowRefused: () => void;
  /** The binding that arms capture, from the action registry. */
  binding: readonly string[];
};

/** Whether the bar has a second row to draw — a refusal, or the run having ended. */
export function hasASecondRow(props: Pick<CaptureBarProps, "serving" | "refused" | "framesRefused">): boolean {
  return !props.serving || props.refused !== undefined || props.framesRefused > 0;
}

export function CaptureBar(props: CaptureBarProps) {
  const { run, address, studio, serving, armed, framesRefused, refused, binding } = props;
  return (
    <div className="armada-capture-bar" data-armed={armed ? "" : undefined}>
      <div className="armada-capture-bar__row">
        <span className="armada-capture-bar__run" title="This window shows a server, not Armada">
          {run}
        </span>
        <span className="armada-capture-bar__address">{address}</span>
        <span className="armada-capture-bar__aim">
          {serving ? `Notes land on ${studio}` : "The run ended"}
        </span>
        <Button variant="ghost" size="sm" disabled={!serving} onClick={props.onReload}>
          Reload
        </Button>
        <Button
          variant={armed ? "primary" : "ghost"}
          size="sm"
          disabled={!serving}
          onClick={() => props.onArm(!armed)}
        >
          {armed ? "Capturing" : "Capture"}
        </Button>
        <KbdChord keys={[...binding]} aria-label={`${binding.join(" ")} turns capturing on`} />
      </div>
      {hasASecondRow(props) ? (
        <div className="armada-capture-bar__said" role="status">
          {serving ? null : (
            <span className="armada-capture-bar__ended">
              This run is no longer serving. What is on screen stays; nothing further loads, and
              capture is closed.
            </span>
          )}
          {refused === undefined ? null : (
            <>
              <span className="armada-capture-bar__refused">
                {refused.said} refused: {refused.address}
              </span>
              {refused.offerable ? (
                <Button variant="ghost" size="sm" onClick={props.onFollowRefused}>
                  Open in browser
                </Button>
              ) : null}
            </>
          )}
          {framesRefused > 0 ? (
            <span className="armada-capture-bar__frames">
              {framesRefused} {framesRefused === 1 ? "frame" : "frames"} off this origin refused
            </span>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
