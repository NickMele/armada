import { ExternalLink } from "lucide-react";
import type { ReactNode } from "react";
import { useState } from "react";

import { Button } from "../../primitives/Button/Button";
import { Checkbox } from "../../primitives/Checkbox/Checkbox";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * The comments people left on this job's pull request, and the ones a drone
 * should act on.
 *
 * **A person chooses, and that is the whole reason this exists.** Not every
 * comment on a pull request is a change request — some are questions, some are
 * agreement, some are about something else entirely — and a drone handed all of
 * them will try to satisfy all of them. So the list is drawn and nothing is
 * preselected.
 *
 * **It sits under the decision, in the same block.** Review and reply are one
 * loop: the diff, then what to do about it, then what other people said about
 * it, one scroll apart and never a second surface.
 *
 * # Every word here except Armada's own was written by somebody outside
 *
 * A comment is written by whoever can see the pull request. It is drawn as a
 * text node and as nothing else — no markdown is rendered, no link is made
 * clickable *out of its own text*, no attribute carries one. What a person
 * does with it is pick it or not, and the only value that goes back on a pick
 * is the handle the forge gave it.
 *
 * # Two things Armada adds, and neither is drawn from the comment's words
 *
 * **The code an inline comment is about** — its file, its line, and the diff
 * around it — comes ahead of the words, exactly as the forge answered it: no
 * syntax highlighting, a `pre` block same as the comment's own paragraphs are
 * `pre-wrap`. **A link to the comment on the forge** is its own control, drawn
 * from `id` through `onOpenLink` rather than from a `url` string handed
 * straight to an anchor — the same discipline `WhereRow`'s `open` act keeps,
 * so nothing this surface was given reaches the OS without main resolving it
 * again.
 *
 * # A comment already sent is drawn and cannot be picked
 *
 * The forge has no memory of what Armada did, so a comment stays on a pull
 * request reading exactly the same forever. One a drone has already been handed
 * is shown with what it says and marked as sent, rather than hidden: hiding it
 * would leave a person looking at a review with holes in it and wondering what
 * happened to the rest. Fleet refuses a press naming one, so offering it as
 * choosable would be offering a press that fails.
 *
 * # No glyph
 *
 * `packages/icons/icons.toml` has no mark for a review comment. `message-*` is
 * not in the registry and nothing else there means it, so this draws none —
 * a state with no glyph gets the words, never an invented mark.
 */
export type ReviewCommentsProps = {
  /** Oldest first, as the forge ordered them. Empty draws `emptyNote`. */
  comments: readonly ReviewComment[];
  /** Send the handles of the ones picked. The caller holds the request. */
  onTakeUp: (ids: string[]) => void;
  /**
   * A press already in flight, or nothing live to send it over. The caller's
   * sentence says which — a disabled control with no reason looks broken.
   */
  disabled?: boolean;
  /** Why the controls are off, where they are. */
  disabledNote?: ReactNode;
  /** The line over the list. Sentence case, no Wh- opener. */
  label?: ReactNode;
  /** What picking commits to, on hover over the line above the list. */
  note?: ReactNode;
  /**
   * What a pull request nobody has commented on says.
   *
   * **Never the sentence for a reading that failed.** Nobody has said anything
   * and nothing could be asked are different facts, and the caller draws the
   * second one instead of this surface.
   */
  emptyNote?: ReactNode;
  takeUpLabel?: string;
  /** What marks a comment a drone has already been handed. */
  sentNote?: string;
  /**
   * Open one comment on the forge. Absent where nothing here can resolve a
   * link, in which case no comment draws the control at all — a link with
   * nothing to open reads as a broken one, not a quiet one.
   */
  onOpenLink?: (id: string) => void;
};

/** One comment, as this surface draws it. */
export type ReviewComment = {
  /** What the forge calls it. Never drawn as text; it is what the press names
   *  and what `onOpenLink` is called with. */
  id: string;
  /** The login of whoever wrote it, as the forge spells it. */
  by: string;
  /** When, already formatted by the caller. Nothing here parses a timestamp. */
  at: string;
  /** What they wrote. */
  said: string;
  /** Whether a drone on this job has already been handed it. */
  takenUp: boolean;
  /**
   * Whether the forge answered an address for this comment. **A boolean, not
   * the address itself** — this surface never holds a URL, so it cannot draw
   * one into anything that would reach the OS.
   */
  hasLink?: boolean;
  /** The code this comment is about, where it is attached to one line of the
   *  diff. Absent for a comment on the pull request's own conversation. */
  inline?: ReviewCommentCode;
};

/** The code one inline comment is about, exactly as the forge answered it. */
export type ReviewCommentCode = {
  /** The file the comment is on, relative to the repository root. */
  path: string;
  /** The line of the current diff the comment sits on. */
  line: number;
  /** The diff around that line, `@@` header included. */
  hunk: string;
};

export function ReviewComments({
  comments,
  onTakeUp,
  disabled = false,
  disabledNote,
  label = "Comments on the pull request",
  note = "Pick the ones a drone should act on. It works on the same branch, so the pull request updates in place, and one reply on it says what was picked up and what was not.",
  emptyNote = "No comments",
  takeUpLabel = "Send to a drone",
  sentNote = "Already sent to a drone",
  onOpenLink,
}: ReviewCommentsProps) {
  // Held here because it is a draft until it is sent, exactly as the review
  // note beside it is. Nothing outside this region knows one is being made.
  const [picked, setPicked] = useState<readonly string[]>([]);
  const choosable = comments.filter((comment) => !comment.takenUp);
  const chosen = picked.filter((id) => choosable.some((comment) => comment.id === id));

  return (
    <section className="armada-remarks" aria-label="Comments on the pull request">
      <div className="armada-remarks__head">
        <Tooltip asChild label={note}>
          <span className="armada-remarks__label">{label}</span>
        </Tooltip>
      </div>

      {comments.length === 0 ? (
        <p className="armada-remarks__said">{emptyNote}</p>
      ) : (
        <>
          <ul className="armada-remarks__list">
            {comments.map((comment) => (
              <li className="armada-remarks__one" key={comment.id}>
                <div className="armada-remarks__who">
                  {/* The login and the instant, both as the forge wrote them.
                      Mono, because they are machine readings and not prose. */}
                  <span className="armada-remarks__by mono">{comment.by}</span>
                  <span className="armada-remarks__at mono">{comment.at}</span>
                  {comment.hasLink && onOpenLink !== undefined ? (
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => onOpenLink(comment.id)}
                      aria-label={`Open ${comment.by}'s comment on the forge`}
                    >
                      <ExternalLink size={12} aria-hidden />
                      Open on the forge
                    </Button>
                  ) : null}
                </div>
                {/* The code this comment is about, ahead of its words: a
                    person reading "this leaks a handle" wants the line it is
                    about in view before the sentence, not after. */}
                {!comment.inline ? null : (
                  <div className="armada-remarks__code">
                    <span className="armada-remarks__file mono">
                      {comment.inline.path}:{comment.inline.line}
                    </span>
                    <pre className="armada-remarks__hunk mono">{comment.inline.hunk}</pre>
                  </div>
                )}
                {/* Somebody else's words, as a text node and nothing else.
                    `pre-wrap` keeps the paragraphs they wrote without rendering
                    a line of it as markup. */}
                <p className="armada-remarks__said armada-remarks__body">{comment.said}</p>
                {comment.takenUp ? (
                  <p className="armada-remarks__sent">{sentNote}</p>
                ) : (
                  <Checkbox
                    checked={chosen.includes(comment.id)}
                    disabled={disabled}
                    onChange={(event) =>
                      setPicked((held) =>
                        event.target.checked
                          ? [...held, comment.id]
                          : held.filter((id) => id !== comment.id),
                      )
                    }
                  >
                    Act on this
                  </Checkbox>
                )}
              </li>
            ))}
          </ul>

          {/* Off until something is picked, for the reason the drone's question
              is: fleet would refuse an empty press, and a round trip to learn
              nothing was chosen is a refusal that reads as a failure. */}
          <Button
            variant="primary"
            disabled={disabled || chosen.length === 0}
            onClick={() => chosen.length > 0 && onTakeUp([...chosen])}
          >
            {takeUpLabel}
          </Button>
        </>
      )}

      {disabled && disabledNote !== undefined ? (
        <p className="armada-remarks__said" role="note">
          {disabledNote}
        </p>
      ) : null}
    </section>
  );
}
