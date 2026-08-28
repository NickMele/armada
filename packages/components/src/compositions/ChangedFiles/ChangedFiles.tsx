import type { MouseEvent, ReactNode } from "react";
import { useCallback } from "react";

/**
 * Changed files — what a Drone has touched in its worktree, as of one reading.
 *
 * **Names and a change kind, never bytes.** What changed inside a file is the
 * patch, which is read only when a Judge fires and is deliberately not on this
 * seam. A file list is the cheapest answer to *is it doing what I asked* —
 * before the gate, before the Judge, before any evidence is submitted — and the
 * diff is a later and more expensive question.
 *
 * **The whole footprint, not a delta.** A reading replaces the list rather than
 * being folded into it, so a file that stopped being changed leaves the view by
 * not being in the next reading. Nothing here accumulates.
 *
 * **Wire order, and no sort.** The reading found them in an order and that
 * order is stable between readings; re-sorting on arrival is the column
 * flip-flop the failure log named, one level down.
 *
 * **No glyph, and the mark column is empty on purpose.** `file` is reserved to
 * the log row and `file-check` to a submission that landed, so a changed-file
 * row has nothing in the registry to take and none is invented here.
 */
export type ChangedFile = {
  /** Repository-relative, exactly as git spells it. Copies on click. */
  path: string;
  /**
   * What happened to it, in the wire's own word: `added`, `modified`,
   * `deleted`, and the rest.
   *
   * **The spelling renders.** `enum-verbs.toml` carries no `change_kind` rows,
   * so there is no verb, glyph or hue for one — a word chosen here would be the
   * second vocabulary the generated module exists to prevent. Reported.
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

  return (
    <div className="armada-files">
      <ol className="armada-files__list">
        {files.map((file) => (
          <li
            className="armada-files__file"
            key={file.path}
            data-outside={file.outsidePlan === true || undefined}
          >
            <span className="armada-files__change">{file.change}</span>
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
