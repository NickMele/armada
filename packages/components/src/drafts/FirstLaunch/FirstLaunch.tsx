import type { ReactNode } from "react";
import {
  BoardEmptyState,
  type BoardEmptyStateProps,
} from "../../compositions/BoardEmptyState/BoardEmptyState";

/**
 * First launch — an empty list, in the two readings it can have.
 *
 * **Not running and unreachable differ on the runtime file.** Fleet writes
 * port, pid and protocol version on startup and removes them on a clean exit,
 * so a missing file is a Fleet that is not there — which is why the second
 * reading can name the command rather than reporting a timeout.
 *
 * The two are drawn side by side because the screen exists to show them
 * together. A surface draws whichever one is true, from the same props.
 */
export type FirstLaunchReading = BoardEmptyStateProps & {
  /** Which reading of an empty list this is, over the card. */
  caption: ReactNode;
};

export type FirstLaunchProps = {
  /** Fleet is up with no work. A null result: the action is the bright thing. */
  running: FirstLaunchReading;
  /** Fleet is not there. A fault Bridge cannot fix, so it names the command. */
  notRunning: FirstLaunchReading;
  /** A clipboard write is silent, so the surface confirms it with a toast. */
  onCopied?: (value: string) => void;
};

export function FirstLaunch({ running, notRunning, onCopied }: FirstLaunchProps) {
  return (
    <div className="armada-screen__row">
      <Reading {...running} onCopied={running.onCopied ?? onCopied} />
      <Reading {...notRunning} onCopied={notRunning.onCopied ?? onCopied} />
    </div>
  );
}

function Reading({ caption, ...state }: FirstLaunchReading) {
  return (
    <div className="armada-screen__card" data-width="half">
      <span className="armada-screen__caption">{caption}</span>
      <BoardEmptyState {...state} />
    </div>
  );
}
