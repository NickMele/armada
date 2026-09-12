import { useState, type ReactNode } from "react";

import { ChevronDown, ChevronRight } from "lucide-react";

/**
 * A step's work, with runs of one tool folded to a line.
 *
 * **A log nobody can scroll is a log nobody reads.** One real step carried 1763
 * rows, 351 of them calls, and drawn flat the reader's own question — what did
 * this Drone actually do — is answered by scrolling past two hundred `Read`
 * lines. Consecutive calls of one tool collapse here to a heading carrying the
 * count and the time they took, which is what every build log that folds
 * anything does.
 *
 * **A group that went wrong is never handed one.** A caller folds what is
 * uneventful; the run holding a failure is the run the reader came for, and a
 * count in front of it is the one thing that must not happen. Such a group
 * arrives with no heading at all and draws its rows bare.
 *
 * **The rows are the caller's.** This composition owns the folding and nothing
 * else — what a row is, how it opens and which keyboard reaches it belong to
 * the log the caller puts in `body`, which is the same log drawn unfolded
 * everywhere else.
 */
export type WorkGroup = {
  /** Stable across re-renders. The first row's own id. */
  id: string;
  /** What the group is: the tool its calls reached for. */
  name: ReactNode;
  /** Whether `name` is machine-derived. Mono names machinery, sans names work. */
  mono?: boolean;
  /** What the folded line says beside the name — `9 calls · 142ms`. */
  meta?: string;
  /**
   * Drawn folded, behind a heading that opens it. **Absent draws the body
   * bare**, which is a group of one and any group a caller refused to fold.
   */
  folded?: boolean;
  /** The rows themselves. */
  body: ReactNode;
};

export type WorkGroupsProps = {
  groups: WorkGroup[];
  /**
   * Rows this Bridge has no drawing for, counted by the wire's own kind.
   *
   * **Counted, never listed, and never silent.** 993 of that step's 1763 rows
   * were these, 757 of them one kind. A log that shortens with nothing saying
   * so reads as a Drone that did less.
   */
  unread?: { kind: string; count: number }[];
  /** What an empty log says. Never a blank: a blank reads as a failed render. */
  emptyNote: string;
};

export function WorkGroups({ groups, unread = [], emptyNote }: WorkGroupsProps) {
  const held = unread.reduce((sum, one) => sum + one.count, 0);
  if (groups.length === 0) {
    return (
      <p className="armada-work__empty" role="note">
        {emptyNote}
      </p>
    );
  }
  return (
    <div className="armada-work">
      {groups.map((group) => (
        <Group key={group.id} group={group} />
      ))}
      {held === 0 ? null : <Unread unread={unread} held={held} />}
    </div>
  );
}

/** One group: a heading that opens it, or the rows themselves. */
function Group({ group }: { group: WorkGroup }) {
  const [open, setOpen] = useState(false);
  if (group.folded !== true) return <>{group.body}</>;
  return (
    <div className="armada-work__group">
      <button
        type="button"
        className="armada-work__head"
        aria-expanded={open}
        onClick={() => setOpen((was) => !was)}
      >
        {open ? (
          <ChevronDown className="armada-work__fold" size={13} strokeWidth={2} aria-hidden />
        ) : (
          <ChevronRight className="armada-work__fold" size={13} strokeWidth={2} aria-hidden />
        )}
        <span className={group.mono === true ? "armada-work__name mono" : "armada-work__name"}>
          {group.name}
        </span>
        {group.meta === undefined ? null : (
          <span className="armada-work__meta">{group.meta}</span>
        )}
      </button>
      {/* Hidden rather than unmounted, the way a chapter hides its body: a row
          a reader opened stays open under a group they folded and reopened. */}
      <div className="armada-work__body" hidden={!open}>
        {group.body}
      </div>
    </div>
  );
}

/** The counted rows, and which kinds they were. */
function Unread({ unread, held }: { unread: { kind: string; count: number }[]; held: number }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="armada-work__group">
      <button
        type="button"
        className="armada-work__head"
        aria-expanded={open}
        onClick={() => setOpen((was) => !was)}
      >
        {open ? (
          <ChevronDown className="armada-work__fold" size={13} strokeWidth={2} aria-hidden />
        ) : (
          <ChevronRight className="armada-work__fold" size={13} strokeWidth={2} aria-hidden />
        )}
        <span className="armada-work__meta">
          {`${held} ${held === 1 ? "row" : "rows"} this Bridge does not draw`}
        </span>
      </button>
      <div className="armada-work__body" hidden={!open}>
        {unread.map((one) => (
          <p key={one.kind} className="armada-work__kind">
            <span className="mono">{one.kind}</span>
            <span className="armada-work__meta">{one.count}</span>
          </p>
        ))}
      </div>
    </div>
  );
}
