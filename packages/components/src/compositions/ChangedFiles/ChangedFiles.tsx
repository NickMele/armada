import type { MouseEvent, ReactNode } from "react";
import { useCallback } from "react";

import { CHANGE_KIND } from "../../generated/vocabulary";

/**
 * Changed files — what a Drone has touched in its worktree, as of one reading.
 *
 * Names and a change kind, never bytes: the patch is read only when a Judge
 * fires, and a file list is the cheapest answer to "is it doing what I
 * asked" before the gate, before the Judge, before any evidence lands. The
 * whole footprint is redrawn each reading rather than a delta, so a file
 * that stopped changing just leaves the view.
 *
 * Wire order, no sort — re-sorting on arrival is the column flip-flop the
 * failure log named. No glyph: `file` is reserved to the log row and
 * `file-check` to a landed submission, so this row has nothing to take.
 * The change-kind word is the registry's (`enum-verbs.toml`'s
 * `change_kind`) — change a row there and this column changes.
 */
export type ChangedFile = {
  /** Repository-relative, exactly as git spells it. Copies on click. */
  path: string;
  /**
   * What happened to it, as the wire spells it: `added`, `modified`,
   * `type_changed`, and the rest.
   *
   * **The wire's value, never a rendered one.** The word on screen is read off
   * `CHANGE_KIND` below, so a caller handing this a word would be the second
   * vocabulary the generated module exists to prevent — and three callers build
   * this row, which is three places one could be typed.
   */
  change: string;
  /**
   * This path is not covered by the plan the step declared.
   *
   * **A mark, not a judgement.** It restates a comparison already made and
   * decides nothing: drift does not fail a step. So the row is lifted onto a
   * surface and says the words, and takes no hue — a drift state is one of the
   * things the design system names as staying neutral below Job level.
   */
  outsidePlan?: boolean;
  /**
   * Lines added and lines removed in this file — the drawing's `+61 −4`.
   * Both optional; a zero is not drawn, since `−0` reads as a value that
   * failed to arrive rather than nothing to report.
   *
   * Nothing on the wire fills this yet: `ChangedFile` (`Saw::Produced`,
   * `job.files_changed`, the finished job's `TouchedFile`) carries a path, a
   * change kind and a drift mark only — the only route with line counts is
   * `get_diff`, the expensive whole-patch read the collapsed chapter exists
   * to avoid. Reported rather than worked around: computing this pair from a
   * fetched patch would spend the read the chapter defers.
   */
  added?: number;
  deleted?: number;
};

export type ChangedFilesProps = {
  /** Every file in the reading, in the order the reading found them. */
  files: ChangedFile[];
  /**
   * What the region says with no files. **Empty is a real answer** — a Drone
   * that has changed nothing yet — and it is not the same sentence as a Job
   * whose footprint nothing serves, so the caller supplies it.
   */
  emptyNote: string;
  /** Where the reading came from, and when. Under the list, never inside it. */
  note?: ReactNode;
  /** A clipboard write is silent, so the surface confirms it with a toast. */
  onCopied?: (value: string) => void;
};

/** What a drifted row says. One wording, so two surfaces cannot disagree. */
const OUTSIDE_PLAN = "outside plan";

/**
 * The word for one change kind — the registry's, never this file's.
 *
 * **Five of the eight rows keep git's own spelling**, so most of the time the
 * verb and the wire value are the same string. `type_changed` and `unreadable`
 * are the two that are not, and they are why this is a lookup rather than a
 * passthrough: the registry argues `could not be read` because what happened is
 * that one path's reading failed, not that the file has a property.
 *
 * **A kind this build's registry has no verb for renders as the wire spelled
 * it** — recoverable, and never copy invented at the call site. That is the
 * same fallback a status with no row takes. #465.
 */
function wordFor(change: string): string {
  return CHANGE_KIND[change]?.verb ?? change;
}

/**
 * The header line for a list of these files — `3 files · +94 −31 · all
 * inside the plan`. It lives beside the list rather than in the surface
 * that draws the header, since a caller spelling its own summary is a
 * second vocabulary that can disagree with the rows.
 *
 * `planDeclared` false means "no step declared one", not "nothing
 * drifted", so the clause is omitted rather than claimed. `all inside the
 * plan` is the drawing's own wording; `2 outside the plan` is decided here
 * to answer it in the same grammar, since the drift counterpart is not
 * drawn anywhere else. Reported.
 */
export function changedFilesSummary(files: ChangedFile[], planDeclared?: boolean): string {
  const parts = [`${files.length} ${files.length === 1 ? "file" : "files"}`];

  // The same `+61 −4` a row draws, over the whole list. One spelling, so the
  // header cannot say `+94 -31` while the rows say `+61 −4`.
  const churn = countsOf({
    added: sum(files, (file) => file.added),
    deleted: sum(files, (file) => file.deleted),
  });
  if (churn !== undefined) {
    parts.push([churn.added, churn.deleted].filter((n) => n !== undefined).join(" "));
  }

  if (planDeclared === true) {
    const outside = files.filter((file) => file.outsidePlan === true).length;
    parts.push(outside === 0 ? "all inside the plan" : `${outside} outside the plan`);
  }

  return parts.join(" · ");
}

/** A total, or nothing where no file in the list carried the count. */
function sum(files: ChangedFile[], of: (file: ChangedFile) => number | undefined): number | undefined {
  const counted = files.map(of).filter((n) => n !== undefined);
  return counted.length === 0 ? undefined : counted.reduce((a, b) => a + b, 0);
}

/**
 * `+61 −4`, `+21`, `−12`, or nothing at all.
 *
 * **A zero is dropped rather than drawn.** `−0` measures nothing and reads as a
 * value that did not arrive; an added file is `+21` in the drawing and nothing
 * sits beside it. U+2212 MINUS SIGN, not a hyphen — it is the glyph every
 * count on this screen already carries.
 */
function countsOf({ added, deleted }: { added?: number; deleted?: number }):
  | { added?: string; deleted?: string }
  | undefined {
  const plus = added === undefined || added === 0 ? undefined : `+${added}`;
  const minus = deleted === undefined || deleted === 0 ? undefined : `\u2212${deleted}`;
  if (plus === undefined && minus === undefined) return undefined;
  return { added: plus, deleted: minus };
}

export function ChangedFiles({ files, emptyNote, note, onCopied }: ChangedFilesProps) {
  const copy = useCallback(
    (event: MouseEvent<HTMLElement>, value: string) => {
      event.stopPropagation();
      void navigator.clipboard.writeText(value).then(() => onCopied?.(value));
    },
    [onCopied],
  );

  if (files.length === 0) {
    return (
      <p className="armada-files__empty" role="note">
        {emptyNote}
      </p>
    );
  }

  // The count track is drawn only where something can fill it. An empty track
  // still takes its column gaps, which would put a double gap in front of the
  // drift mark on every list — and no list can fill it today.
  const counted = files.some((file) => countsOf(file) !== undefined);

  return (
    <div className="armada-files">
      <ol className="armada-files__list" data-counts={counted || undefined}>
        {files.map((file) => (
          <li
            className="armada-files__file"
            key={file.path}
            data-outside={file.outsidePlan === true || undefined}
          >
            <span className="armada-files__change">{wordFor(file.change)}</span>
            {/* The title carries the whole path however narrow the row gets,
                and so does the clipboard: a copy that truncated with the
                display would be worse than the overflow it was fixing. */}
            <span
              className="armada-files__path"
              title={file.path}
              onClick={(event) => copy(event, file.path)}
            >
              {file.path}
            </span>
            {counted ? <Counts file={file} /> : null}
            <span className="armada-files__mark">
              {file.outsidePlan === true ? OUTSIDE_PLAN : null}
            </span>
          </li>
        ))}
      </ol>
      {note === undefined ? null : <p className="armada-files__note">{note}</p>}
    </div>
  );
}

/**
 * One file's `+61 −4`. An empty cell where this file carries neither, because
 * the track is declared for the list and every row borrows it.
 */
function Counts({ file }: { file: ChangedFile }) {
  const counts = countsOf(file);
  if (counts === undefined) return <span className="armada-files__counts" />;
  return (
    <span className="armada-files__counts">
      {counts.added === undefined ? null : (
        <span className="armada-files__added">{counts.added}</span>
      )}
      {counts.deleted === undefined ? null : (
        <span className="armada-files__deleted">{counts.deleted}</span>
      )}
    </span>
  );
}
