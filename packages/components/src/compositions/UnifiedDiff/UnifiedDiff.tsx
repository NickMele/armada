import type { MouseEvent, ReactNode } from "react";
import { useCallback } from "react";

/**
 * Unified diff — what moved inside the files, as the repository rendered it.
 *
 * **The expensive read, drawn in the one place it is worth paying for.**
 * `ChangedFiles` answers *is it doing what I asked* from names alone, which is
 * what a running Job wants. This answers *do I take this work*, and nothing
 * short of the bytes answers that. `crates/adapter-traits/src/work_product.rs`
 * separates the patch from the file list for exactly that reason.
 *
 * **The marker renders, not just the hue.** A `+` and a `-` are what the line
 * is; colour restates it. A diff encoded only in colour is the illegible legend
 * the v1 failure log named, and it is unreadable to a reader who cannot tell
 * `--diff-add-fg` from `--diff-del-fg` at all.
 *
 * **No line numbers, and none are computed.** The hunk header is git's own
 * statement of position and renders as its own row; per-line numbering would be
 * derived here from that header, and a derived number sitting beside machine
 * output reads as something the repository said.
 *
 * **Wire order, and no sort.** Files come in the order the reading found them,
 * hunks in the order git wrote them. Re-ordering on arrival is the column
 * flip-flop the failure log named, one level down.
 *
 * **It is bounded, and it says so when it cut.** No virtualization library is
 * chosen — `docs/practices/bridge.md`, `[list-virtualization]` — so a patch too
 * long to draw is cut rather than rendered whole, and `cut` is what the surface
 * says about it. **A cut diff is a decision made on part of the work**, so the
 * sentence has to name the worktree rather than trail off.
 */
export type DiffLine = {
  /**
   * `hunk` is git's `@@` header; the other three are what the line did.
   *
   * There is no `meta` kind: `index`, `new file mode` and the rest belong to
   * the file rather than to a line, and they arrive on `DiffFile.meta`.
   */
  kind: "hunk" | "added" | "removed" | "context";
  /** The line exactly as git wrote it, marker included. Never re-marked here. */
  text: string;
};

export type DiffFile = {
  /** Repository-relative, exactly as git spells it. Copies on click. */
  path: string;
  /**
   * What git said about the file that is not a line of it — `new file mode
   * 100644`, a rename's old name. Mono, beside the path.
   */
  meta?: ReactNode;
  /**
   * This path is not covered by the plan the step declared. **A mark, not a
   * judgement**, and the same wording `ChangedFiles` uses — one vocabulary for
   * one fact, so a file that reads drifted in the list does not read clean
   * here.
   */
  outsidePlan?: boolean;
  lines: DiffLine[];
};

export type UnifiedDiffProps = {
  /** Every file in the patch, in the order the reading found them. */
  files: DiffFile[];
  /**
   * What the region says with no patch. **Three different silences** — a Job
   * with no worktree, a Drone that changed nothing, and a read that failed —
   * so the caller supplies the sentence rather than sharing one here.
   */
  emptyNote: string;
  /**
   * What was left undrawn, where the patch was longer than the bound. **Loud,
   * and it names where the rest is**: a decision taken on a diff that quietly
   * stopped is the failure this whole surface exists to prevent.
   */
  cut?: ReactNode;
  /** Where the reading came from. Under the diff, never inside it. */
  note?: ReactNode;
  /** A clipboard write is silent, so the surface confirms it with a toast. */
  onCopied?: (value: string) => void;
};

/** What a drifted file says. `ChangedFiles`' wording, not a second one. */
const OUTSIDE_PLAN = "outside plan";

export function UnifiedDiff({ files, emptyNote, cut, note, onCopied }: UnifiedDiffProps) {
  const copy = useCallback(
    (event: MouseEvent<HTMLElement>, value: string) => {
      event.stopPropagation();
      void navigator.clipboard.writeText(value).then(() => onCopied?.(value));
    },
    [onCopied],
  );

  if (files.length === 0) {
    return (
      <p className="armada-diff__empty" role="note">
        {emptyNote}
      </p>
    );
  }

  return (
    <div className="armada-diff">
      {cut === undefined ? null : (
        <p className="armada-diff__cut" role="note">
          {cut}
        </p>
      )}
      {files.map((file) => (
        <section className="armada-diff__file" key={file.path}>
          <header className="armada-diff__head" data-outside={file.outsidePlan === true || undefined}>
            {/* The title carries the whole path however narrow the pane gets,
                and so does the clipboard: a copy that truncated with the
                display would be worse than the overflow it was fixing. */}
            <span
              className="armada-diff__path"
              title={file.path}
              onClick={(event) => copy(event, file.path)}
            >
              {file.path}
            </span>
            {file.meta === undefined ? null : (
              <span className="armada-diff__meta">{file.meta}</span>
            )}
            <span className="armada-diff__mark">
              {file.outsidePlan === true ? OUTSIDE_PLAN : null}
            </span>
          </header>
          {/* `pre` per line rather than one block: a hunk header and a changed
              line take different surfaces, and a single block cannot carry
              them. Nothing wraps — a diff that reflowed would move a marker
              off the column it identifies the line by. */}
          <ol className="armada-diff__lines">
            {file.lines.map((line, i) => (
              <li className="armada-diff__line" data-kind={line.kind} key={i}>
                <pre className="armada-diff__text">{line.text}</pre>
              </li>
            ))}
          </ol>
        </section>
      ))}
      {note === undefined ? null : <p className="armada-diff__note">{note}</p>}
    </div>
  );
}
