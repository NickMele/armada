import type { MouseEvent, ReactNode } from "react";
import { useCallback } from "react";

/**
 * Board empty state — one line, and the thing to do about it.
 *
 * **No centred glyph and no illustration.** The empty state points at the work
 * available and nothing else: this is an instrument panel, and empty-state art
 * is the decoration hard rule six rules out. What is left is a sentence, an
 * optional command, and at most one control.
 *
 * **The two states differ because the two situations do.** A Fleet that is up
 * with no work is a null result — nothing is wrong, so the line is set back and
 * the action beneath it is the only bright thing. A Fleet that is not running
 * is a fault Bridge cannot fix: the line states it flatly, and what follows is
 * the command a person runs in a terminal. Rendering the two alike would make a
 * connection Bridge is waiting on look like a list that happens to be empty.
 *
 * **The command is a value, not a button.** Bridge does not start Fleet at this
 * milestone, so a button offering to would be a control that cannot act. It is
 * machine-derived, so it is mono, and like every machine value it copies on
 * click and carries no `copy` glyph.
 */
export type BoardEmptyStateProps = {
  /** The one line. The whole reading, before anything beneath it. */
  children: ReactNode;
  /**
   * Set back to `--fg-muted`, where the empty state is a null result rather
   * than a fault: nothing is wrong and the action is what the eye should
   * reach.
   */
  quiet?: boolean;
  /**
   * A command to run elsewhere, in mono. It copies on click and goes to
   * `--accent` on hover — it is a value, and Bridge cannot run it.
   */
  command?: string;
  /**
   * The sentence under the command: what to do with it, and what happens next
   * without any further action from the person.
   */
  note?: ReactNode;
  /** The one control, where the state has an act available. */
  action?: ReactNode;
  /** A clipboard write is silent, so the surface confirms it with a toast. */
  onCopied?: (value: string) => void;
};

export function BoardEmptyState({
  children,
  quiet = false,
  command,
  note,
  action,
  onCopied,
}: BoardEmptyStateProps) {
  const copy = useCallback(
    (event: MouseEvent<HTMLSpanElement>, value: string) => {
      event.stopPropagation();
      void navigator.clipboard.writeText(value).then(
        () => onCopied?.(value),
        // A failed clipboard write is otherwise indistinguishable from a dead
        // element, so the surface is told either way.
        () => onCopied?.(value),
      );
    },
    [onCopied],
  );

  return (
    <div className="armada-board-empty">
      <span className="armada-board-empty__line" data-quiet={quiet || undefined}>
        {children}
      </span>
      {command !== undefined ? (
        <span className="armada-board-empty__command" onClick={(e) => copy(e, command)}>
          {command}
        </span>
      ) : null}
      {note ? <span className="armada-board-empty__note">{note}</span> : null}
      {action}
    </div>
  );
}
