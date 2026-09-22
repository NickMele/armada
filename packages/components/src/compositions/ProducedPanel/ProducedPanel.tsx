import { useState, type ReactNode } from "react";

import { CHANGE_KIND } from "../../generated/vocabulary";
import { Chapter } from "../Chapter/Chapter";

/**
 * What the Job has changed, as a panel of its own beside the run. #1187.
 *
 * **The Job's and not the step's.** It used to be a section inside the step's
 * Working card, and that card carried two shortcuts because it was doing two
 * jobs — the owner's call, 15 Sep 2026. Working keeps the work and `Open the
 * log`; this keeps the change and `Open the diff`.
 *
 * **A `Chapter`, like every phase above it**, so the card, the header line, the
 * act and the body's well are the ones already agreed.
 */
export type ProducedPanelProps = {
  /** `8 files · +154 −23`, the header's fact. */
  summary?: ReactNode;
  /** `Open the diff`, where there is a patch to open. */
  act?: ReactNode;
  /** The change summary, then anything kept beside the diff. */
  children?: ReactNode;
};

export function ProducedPanel({ summary, act, children }: ProducedPanelProps) {
  const [open, setOpen] = useState(true);
  return (
    <div className="armada-produced">
      <Chapter
        name="Produced"
        {...(summary === undefined ? {} : { meta: summary })}
        {...(act === undefined ? {} : { act })}
        open={open}
        onToggle={() => setOpen(!open)}
      >
        {children}
      </Chapter>
    </div>
  );
}

/** One file in the change summary. */
export type ProducedFile = {
  /** Repository-relative, whole. What a hover shows. */
  path: string;
  /** The path under its folder — `daemon/fittings.rs`. */
  name: string;
  /** The wire's change kind. `added` is marked `new`; `modified` is not marked. */
  change: string;
  /** Absent where nothing counted the file, which is not zero. */
  added?: number;
  deleted?: number;
  /** Not covered by the plan the step declared. A mark, not a judgement. */
  outsidePlan?: boolean;
};

/** One folder, with the lines its files gained and lost between them. */
export type ProducedFolder = {
  /** `crates/fleet/src`, or `.` for the top of the repository. */
  path: string;
  added?: number;
  deleted?: number;
  /** Biggest change first. */
  files: ProducedFile[];
};

export type ChangeSummaryProps = {
  /** Biggest folder first. The caller groups and orders; this draws. */
  folders: ProducedFolder[];
  /** `and 6 more files`, where the caller drew fewer than there are. */
  more?: string;
  /** What a reading with no files says. Never a blank. */
  emptyNote: string;
  /** Under the list: where it went outside a plan. */
  note?: ReactNode;
};

/**
 * The Job's real diff by folder: each file with `+31 −9` and a bar for its
 * size against the biggest file drawn, new files marked.
 *
 * **The bar is neutral.** Diff colour on it, and the running colour around it,
 * are #1196's.
 */
export function ChangeSummary({ folders, more, emptyNote, note }: ChangeSummaryProps) {
  const files = folders.flatMap((folder) => folder.files);
  if (files.length === 0) {
    return (
      <p className="armada-produced__empty" role="note">
        {emptyNote}
      </p>
    );
  }
  const biggest = Math.max(1, ...files.map((file) => (file.added ?? 0) + (file.deleted ?? 0)));
  return (
    <div className="armada-produced__summary">
      {folders.map((folder) => (
        <section className="armada-produced__folder" key={folder.path} aria-label={folder.path}>
          <p className="armada-produced__folder-name">
            <span className="mono">{folder.path}</span>
            <Counts added={folder.added} deleted={folder.deleted} />
          </p>
          <ul className="armada-produced__files">
            {folder.files.map((file) => (
              <li className="armada-produced__file" key={file.path} title={file.path}>
                <span className="armada-produced__name">
                  <span className="mono">{file.name}</span>
                  {markOf(file.change) === undefined ? null : (
                    <span className="armada-produced__tag">{markOf(file.change)}</span>
                  )}
                  {file.outsidePlan === true ? (
                    <span className="armada-produced__tag">{OUTSIDE_PLAN}</span>
                  ) : null}
                </span>
                <Size added={file.added} deleted={file.deleted} biggest={biggest} />
                <Counts added={file.added} deleted={file.deleted} />
              </li>
            ))}
          </ul>
        </section>
      ))}
      {more === undefined ? null : <p className="armada-produced__more">{more}</p>}
      {note === undefined ? null : <p className="armada-produced__note">{note}</p>}
    </div>
  );
}

/** What a drifted row says. `ChangedFiles`'s words, so the two cannot disagree. */
const OUTSIDE_PLAN = "outside plan";

/**
 * The word beside a file's name. **`new` for an added file**, as the frames
 * drew it; nothing for a modified one, which is most of them; and the
 * registry's own verb for every other kind.
 */
function markOf(change: string): string | undefined {
  if (change === "modified") return undefined;
  if (change === "added") return "new";
  return CHANGE_KIND[change]?.verb ?? change;
}

/** `+31 −9`, dropping a zero, or an empty cell where nothing counted it. */
function Counts({ added, deleted }: { added?: number; deleted?: number }) {
  return (
    <span className="armada-produced__counts mono">
      {added === undefined || added === 0 ? null : (
        <span className="armada-produced__added">{`+${added}`}</span>
      )}
      {deleted === undefined || deleted === 0 ? null : (
        <span className="armada-produced__deleted">{`−${deleted}`}</span>
      )}
    </span>
  );
}

/**
 * A file's size against the biggest drawn, as grown segments rather than
 * widths: added, removed, and the rest of the track. Hidden from a reader,
 * because the counts beside it already say it in words.
 */
function Size({ added = 0, deleted = 0, biggest }: { added?: number; deleted?: number; biggest: number }) {
  return (
    <span className="armada-produced__size" aria-hidden>
      <span className="armada-produced__size-added" style={{ flexGrow: added }} />
      <span className="armada-produced__size-deleted" style={{ flexGrow: deleted }} />
      <span style={{ flexGrow: Math.max(0, biggest - added - deleted) }} />
    </span>
  );
}

/**
 * One group of the plan, once the Job is over. #1542.
 *
 * **The group is what a landed Job is read by**, not the task: the Checks run
 * at a group's end, the commit is the group's and so is the retry — #1530,
 * 21 Sep. A task's own figures are the Plan's to draw.
 */
export type ProducedGroup = {
  /** `Group one`. The caller words it; this draws it. */
  name: string;
  /** The verb for where the group got to — `passed`, `landed`, `failed`. */
  verb: string;
  /** The status token stem that verb takes. `Badge`'s own field. */
  status?: string;
  /** `2 of 2 done`, from the tasks the group holds. */
  tasks: string;
  /**
   * How long the group took. **Absent where nothing timed it**, which is every
   * group today: the wire times a step and has no group to time. A caller says
   * so in `note` rather than drawing a zero.
   */
  took?: string;
  /** How many files its tasks claim. */
  files: string;
  /** What ran at its boundary — `7 Checks, run twice`. */
  checks?: string;
  /** The commit it left, where it left one. Mono, and it copies. */
  commit?: string;
};

export type ProducedGroupsProps = {
  groups: ProducedGroup[];
  /** What a Job with no groups says. Never an empty table. */
  emptyNote: string;
  /** Under the list — what nothing timed, and anything else the rows cannot say. */
  note?: ReactNode;
};

/**
 * The groups a Job ran, one row each: what it came to, its tasks, how long it
 * took, what it wrote and what it left.
 *
 * **A row says what it cannot say.** A group nothing timed draws the word for
 * that in the column, rather than a dash a reader has to interpret or a zero
 * that claims a measurement.
 */
export function ProducedGroups({ groups, emptyNote, note }: ProducedGroupsProps) {
  if (groups.length === 0) {
    return (
      <p className="armada-produced__empty" role="note">
        {emptyNote}
      </p>
    );
  }
  return (
    <div className="armada-produced__groups">
      <ol className="armada-produced__group-rows">
        {groups.map((group) => (
          <li className="armada-produced__group" key={group.name}>
            <span className="armada-produced__group-name">{group.name}</span>
            <span
              className="armada-produced__group-verb"
              {...(group.status === undefined
                ? {}
                : { style: { color: `var(--status-${group.status})` } })}
            >
              {group.verb}
            </span>
            <span className="armada-produced__group-fact">{group.tasks}</span>
            <span className="armada-produced__group-fact">{group.took ?? NOT_TIMED}</span>
            <span className="armada-produced__group-fact">{group.files}</span>
            <span className="armada-produced__group-fact">{group.checks}</span>
            <span className="armada-produced__group-commit mono">{group.commit}</span>
          </li>
        ))}
      </ol>
      {note === undefined ? null : <p className="armada-produced__note">{note}</p>}
    </div>
  );
}

/**
 * What a group nothing timed reads as.
 *
 * **Not a dash and not a zero.** The wire times a step, and a group is between
 * a step and a task, so there is no instant to subtract — which is a fact about
 * what Fleet records rather than a group that took no time.
 */
const NOT_TIMED = "not timed";
