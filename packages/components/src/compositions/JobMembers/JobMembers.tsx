import { GitBranch, GitPullRequest } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { MouseEvent, ReactNode } from "react";
import { useCallback, useState } from "react";

import type { JudgeAnswer } from "@armada/protocol";
import { Badge } from "../../primitives/Badge/Badge";
import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";
import { JudgeQuestion } from "../JudgeQuestion/JudgeQuestion";

/**
 * How a member's work reaches the one before it.
 * `docs/concepts/landing.md` defines the three and nothing else may.
 */
export type JobMemberLink = "stacked" | "merged" | "published";

/**
 * Which link a member carries, written once.
 *
 * **Fixed copy, identical on every member that carries it** — the Voice
 * contract's rule for Armada's own strings.
 *
 * **Each says which of the three this member is and stops there.** What the
 * three *are* — that stacked keeps working and rebases, that parked does not,
 * that published waits on a release rather than on the merge — is true of a
 * member that has never run, so it is guide 2 and the `?` over the list
 * (#1602). The sentences here used to carry both.
 */
export const MEMBER_LINK: Readonly<Record<JobMemberLink, string>> = {
  stacked: "Stacked on the one before it.",
  merged: "Parked until the one before it lands.",
  published: "Waits on what the one before it publishes.",
};

/** The button, and the word it produces. `design-system.md`, the verb table. */
export const DROP_MEMBER_LABEL = "Drop";

/**
 * A member's state, as the registry renders it.
 *
 * **Two shapes, because a status the registry has no verb or glyph for must
 * not be drawn as a badge with something invented in it.** `text` renders the
 * wire spelling and says what is missing; a blank cell hides both.
 */
export type JobMemberState =
  | { as: "badge"; status: string; icon: LucideIcon; label: string }
  | { as: "text"; wire: string; missing: string };

/** A Judge refusal one member is holding, answered where it is read. */
export type JobMemberQuestion = {
  /** Which criterion refused. */
  question: string;
  expected: string;
  produced: string;
  consequence: string;
  /**
   * What the answer moves besides this member — the pull requests held behind
   * it, named. **Spelled out rather than implied**: the same verdict is
   * written either way, and what is only true here is that others are waiting
   * on the press.
   */
  moves: ReactNode;
  onAnswer: (answer: JudgeAnswer, note?: string) => void;
  /** An answer already in flight, or nothing live to send it over. */
  disabled?: boolean;
  /** Why the controls are off, where they are. */
  disabledNote?: string;
  /** The answer that was pressed and Fleet has not answered. */
  pending?: boolean;
};

/** One Job landing under this one. */
export type JobMemberRow = {
  id: string;
  /** Where it sits in the order, counting from one. */
  ordinal: number;
  title: string;
  state: JobMemberState;
  /**
   * Whether its pull request merged. **The merge, not a successful status** —
   * a Job whose gate hands off to a person finishes before anyone merges it.
   */
  landed?: boolean;
  /** How it reaches the member before it. Absent on the first, which has none. */
  link?: JobMemberLink;
  /** The branch its pull request targets, where it has one to target. */
  targets?: string;
  /** The address of its pull request. Absent until it has opened one. */
  pullRequest?: string;
  /** What that address is called on screen — `1591`, never the whole URL. */
  pullRequestLabel?: string;
  branch?: string;
  /** The paths it writes. */
  scope?: readonly string[];
  /** Its plan, counted — `3 of 4`. */
  tasks?: string;
  /** Why somebody dropped it. Present only on a dropped member. */
  dropped?: string;
  question?: JobMemberQuestion;
  /** Open this member's own Job. */
  onOpen?: () => void;
  /**
   * Drop it: close its pull request, keep its branch, and rebase whatever was
   * stacked on it onto the one before. **Absent draws no control** — a member
   * that already landed or was already dropped has nothing to drop.
   */
  onDrop?: (reason: string) => void;
  /**
   * Open the pull request where it lives. **Bound by the caller and taking no
   * address**: what is sent is the member's Job id, so the string this card
   * drew never decides what opens. Nothing in Bridge navigates.
   */
  onOpenPullRequest?: () => void;
};

export type JobMembersProps = {
  /** Every member, in the order they land. */
  members: readonly JobMemberRow[];
  /**
   * What has to happen before this Job is finished, and how far along that is
   * — `Done when every member has landed. One of three pull requests is in.`
   */
  completeWhen: ReactNode;
  /** Why there are no members, where there are none. Said in words. */
  absent?: ReactNode;
  /** A sentence the surface says once, briefly — `Dropped`. */
  onSaid?: (sentence: string) => void;
  /** A clipboard write is silent, so the surface confirms it. */
  onCopied?: (value: string) => void;
};

/** Row marks are 12px at strokeWidth 2, like every mark below Job level. */
const ROW_ICON = 12;
const ROW_STROKE = 2;

/**
 * A Job whose members are Jobs — several pull requests, landing in a set order.
 *
 * **The screen says what it is and never names a shape** (#1530, 22 Sep).
 * Convoy, Train and Atomic are retired; the words here are "pull requests,
 * landing in order" and "member".
 *
 * **The order is the point**, so the ordinal and the rail between cards carry
 * it rather than leaving a reader to infer it from dates, and the link sits at
 * the top of the card it belongs to.
 *
 * **Landed means the pull request merged**, never `completed_success`: a Job
 * handed off to a person finishes while its pull request is still open, and
 * counting those would say the change is in when none of it is.
 */
export function JobMembers({ members, completeWhen, absent, onSaid, onCopied }: JobMembersProps) {
  if (members.length === 0) {
    return (
      <p className="armada-members__absent" role="note">
        {absent ?? "No Job lands under this one."}
      </p>
    );
  }
  return (
    <div className="armada-members">
      <p className="armada-members__complete">{completeWhen}</p>
      {/* The label is the list's, not a wrapper's: what a reader is being
          handed is an ordered set of pull requests, and the order is the
          fact the name has to carry. */}
      <ol className="armada-members__list" aria-label="Pull requests, in the order they land">
        {members.map((member) => (
          <Member key={member.id} member={member} onSaid={onSaid} onCopied={onCopied} />
        ))}
      </ol>
    </div>
  );
}

function Member({
  member,
  onSaid,
  onCopied,
}: {
  member: JobMemberRow;
  onSaid?: (sentence: string) => void;
  onCopied?: (value: string) => void;
}) {
  const copy = useCallback(
    (event: MouseEvent<HTMLElement>, value: string) => {
      event.stopPropagation();
      void navigator.clipboard.writeText(value).then(() => onCopied?.(value));
    },
    [onCopied],
  );
  const address = member.pullRequest;
  const branch = member.branch;

  return (
    <li
      className="armada-members__member"
      data-dropped={member.dropped === undefined ? undefined : true}
      aria-label={`${member.ordinal}. ${member.title}`}
    >
      {/* The order, and the rail that joins one card to the next. Numbered
          rather than inferred: what a reader has to hold is which lands first. */}
      <span className="armada-members__rail" aria-hidden>
        <span className="armada-members__ordinal">{member.ordinal}</span>
      </span>

      <div className="armada-members__body">
        <div className="armada-members__head">
          {member.onOpen === undefined ? (
            <span className="armada-members__title">{member.title}</span>
          ) : (
            <button type="button" className="armada-members__open" onClick={member.onOpen}>
              {member.title}
            </button>
          )}
          {member.state.as === "badge" ? (
            // The running mark pulses on every running row of a list (#1276),
            // and `Badge` is what decides which status that is.
            <Badge status={member.state.status} icon={member.state.icon} pulsing>
              {member.state.label}
            </Badge>
          ) : (
            <span className="armada-members__state-text" title={member.state.missing}>
              {member.state.wire}
            </span>
          )}
        </div>

        {/* The link belongs to the edge above this card, so it reads before
            the card's own facts. */}
        {member.link === undefined ? null : (
          <p className="armada-members__waits">{MEMBER_LINK[member.link]}</p>
        )}
        {member.landed === true ? (
          <p className="armada-members__landed">Its pull request merged.</p>
        ) : null}
        {member.dropped === undefined ? null : (
          <p className="armada-members__dropped">Dropped. {member.dropped} Its branch is kept.</p>
        )}

        <dl className="armada-members__facts">
          <Fact
            name="Pull request"
            icon={GitPullRequest}
            iconLabel="Pull request"
            absent="None opened yet."
            value={
              address === undefined ? undefined : (
                <a
                  href={address}
                  className="armada-members__address"
                  onClick={(event) => {
                    event.preventDefault();
                    member.onOpenPullRequest?.();
                  }}
                >
                  {member.pullRequestLabel ?? address}
                </a>
              )
            }
          />
          <Fact
            name="Branch"
            icon={GitBranch}
            iconLabel="Branch"
            absent="No worktree yet."
            value={
              branch === undefined ? undefined : (
                <button
                  type="button"
                  className="armada-members__copy"
                  title="Copy"
                  onClick={(event) => copy(event, branch)}
                >
                  {branch}
                </button>
              )
            }
          />
          <Fact
            name="Targets"
            absent="Nowhere yet. It has no pull request."
            value={member.targets === undefined ? undefined : <code>{member.targets}</code>}
          />
          <Fact
            name="Writes"
            absent="Nothing declared."
            value={
              member.scope === undefined || member.scope.length === 0 ? undefined : (
                <span className="armada-members__scope">
                  {member.scope.map((path) => (
                    <code key={path}>{path}</code>
                  ))}
                </span>
              )
            }
          />
          <Fact name="Tasks" absent="No plan of its own." value={member.tasks} />
        </dl>

        {member.question === undefined ? null : (
          <div className="armada-members__question">
            <p className="armada-members__moves">{member.question.moves}</p>
            <JudgeQuestion
              question={member.question.question}
              expected={member.question.expected}
              produced={member.question.produced}
              consequence={member.question.consequence}
              onAnswer={member.question.onAnswer}
              {...(member.question.disabled === undefined
                ? {}
                : { disabled: member.question.disabled })}
              {...(member.question.disabledNote === undefined
                ? {}
                : { disabledNote: member.question.disabledNote })}
              {...(member.question.pending === undefined
                ? {}
                : { pending: member.question.pending })}
            />
          </div>
        )}

        {member.onDrop === undefined || member.dropped !== undefined ? null : (
          <DropMember member={member} onSaid={onSaid} />
        )}
      </div>
    </li>
  );
}

/** One fact about a member. A part nothing serves keeps its row and says so. */
function Fact({
  name,
  icon: Icon,
  iconLabel,
  value,
  absent,
}: {
  name: string;
  icon?: LucideIcon;
  iconLabel?: string;
  value?: ReactNode;
  absent: ReactNode;
}) {
  return (
    <div className="armada-members__fact">
      <dt className="armada-members__fact-name">
        {Icon === undefined ? null : (
          <Icon size={ROW_ICON} strokeWidth={ROW_STROKE} aria-label={iconLabel} />
        )}
        {name}
      </dt>
      <dd
        className="armada-members__fact-value"
        data-absent={value === undefined ? "true" : undefined}
      >
        {value ?? absent}
      </dd>
    </div>
  );
}

/**
 * Drop a member: close its pull request, keep its branch, and rebase whatever
 * was stacked on it onto the one before.
 *
 * **The reason is inline, not a dialog** — the plan's own drop settled that: a
 * modal over the list would hide the member the reason is about.
 */
function DropMember({
  member,
  onSaid,
}: {
  member: JobMemberRow;
  onSaid?: (sentence: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [reason, setReason] = useState("");
  // A pristine field is a hint, never an error: a red border on a field
  // nobody has touched reads as a mistake that already happened.
  const [attemptedEmpty, setAttemptedEmpty] = useState(false);
  const blank = reason.trim() === "";

  function close(): void {
    setOpen(false);
    setReason("");
    setAttemptedEmpty(false);
  }

  function drop(): void {
    if (blank) {
      setAttemptedEmpty(true);
      return;
    }
    member.onDrop?.(reason.trim());
    onSaid?.("Dropped");
    close();
  }

  if (!open) {
    return (
      <div className="armada-members__acts">
        <Button variant="secondary" size="sm" ground="sunken" onClick={() => setOpen(true)}>
          {DROP_MEMBER_LABEL}…
        </Button>
      </div>
    );
  }

  return (
    <div className="armada-members__drop">
      <p className="armada-members__drop-said">
        Its pull request is closed and its branch is kept. Whatever is stacked on it rebases onto
        the one before.
      </p>
      <Input
        label="Reason"
        value={reason}
        invalid={attemptedEmpty && blank}
        onChange={(event) => setReason(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Escape") close();
          if (event.key === "Enter") drop();
        }}
      />
      {blank ? (
        <span className="armada-members__hint" data-tone={attemptedEmpty ? "error" : "muted"}>
          A reason is needed.
        </span>
      ) : null}
      <div className="armada-members__acts">
        <Button variant="secondary" size="sm" ground="sunken" onClick={close}>
          Cancel
        </Button>
        <Button variant="secondary" size="sm" ground="sunken" disabled={blank} onClick={drop}>
          {DROP_MEMBER_LABEL}
        </Button>
      </div>
    </div>
  );
}
