import { useState } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Checkbox } from "../../primitives/Checkbox/Checkbox";
import { Dialog } from "../../primitives/Dialog/Dialog";
import { COPY_THE_ISSUE, copyIssue, FILE_AN_ISSUE, issueBody, NO_SCRUB, OPENS_NOTHING } from "./issue";
import type { Filing } from "./issue";

// The producer is re-exported through the control it belongs to, the way the
// payload is re-exported through the notice. Neither is a component and
// neither appears on its own.
export type { Attachment, Filing, Withheld } from "./issue";
export {
  COPIED_ISSUE,
  COPY_THE_ISSUE,
  copyIssue,
  envelopeOf,
  FILE_AN_ISSUE,
  issueBody,
  NO_SCRUB,
  NOT_OFFERED,
  OPENS_NOTHING,
} from "./issue";

/** The fold's chevrons are chrome, so 16px at strokeWidth 2 like every other. */
const FOLD_ICON = 16;
const FOLD_STROKE = 2;

/** What the control on a row that cannot be removed says instead of a checkbox. */
const ALWAYS = "Always sent";

/** How the fold offering an item's text is named. The drawing's word. */
const READ = "Read";

export type FileAnIssueProps = {
  /**
   * What could leave, composed at the moment the review opens.
   *
   * **A function rather than a value, because the payload is stamped when
   * somebody goes to read it.** `at` says when the artifact was *taken*, and a
   * failure notice is a standing condition redrawn every second — a value
   * rebuilt on each render would tick under the reader while the dialog was
   * open, and what is on screen has to be what arrives in the issue body.
   *
   * `attached` here is everything offered, in the order it will be read. **Never
   * a list the caller filtered first** — an item taken out before the dialog
   * opened is an item nobody decided about, and this surface exists to be where
   * that decision is made.
   */
  compose: () => Filing;
  /** A clipboard write is silent, so the surface confirms it. */
  onCopied?: (what: string) => void;
};

/**
 * File an issue — the control, and the review that stands between it and the
 * clipboard.
 *
 * **Send is never one press from the error.** The control opens this and does
 * nothing else; the artifact is composed on the second press. That is why the
 * button is offered where the payload is already legible in full — a
 * full-surface error and an expanded view — and nowhere somebody is reading one
 * line of it. A toast cannot host a review, and an inline error has no room
 * for one.
 *
 * # The review is per item, and one item cannot be removed
 *
 * The error's own record is required, because an issue with it taken out is a
 * sentence somebody typed — which is what this replaces. **Required is not a
 * safety claim.** The drawing locked the envelope on because it was
 * "structured and bounded; cannot carry a credential", and that is true of the
 * structured fields and false of the artifact: `message` and `chain` are prose
 * written by whatever raised the error. So the row that cannot be taken out
 * carries the same warning as every other, and its text is on screen in full —
 * an item a person cannot remove is the item they had most better have read.
 *
 * # No claim is made about what any of it holds
 *
 * Armada makes no scrub claim, says so above the rows, and puts the exact text
 * of each item behind a **Read** on its own row. A promise it cannot keep is
 * worse than the work of reading.
 *
 * # Nothing here sends
 *
 * The confirm copies. `issue.ts` carries why there is no transport, and what
 * the drawing asked for that therefore is not built.
 */
export function FileAnIssue({ compose, onCopied }: FileAnIssueProps) {
  /**
   * What is under review, frozen at the moment the control was pressed. `null`
   * is closed — one piece of state rather than a flag beside a value, so there
   * is no arrangement in which the dialog is open over a filing composed for a
   * failure somebody has already left.
   */
  const [offered, setOffered] = useState<Filing | null>(null);
  /**
   * What has been taken out. **Removals rather than selections**, so a default
   * belongs to the item and an item added later arrives on rather than
   * silently off.
   */
  const [removed, setRemoved] = useState<ReadonlySet<string>>(new Set());

  function close(): void {
    setOffered(null);
    setRemoved(new Set());
  }

  return (
    <>
      {/* Ghost, like every other control in the error treatment: none of them
          is a decision Armada participates in. No glyph, for the reason the
          treatment gives — it carries none, and both alarm marks are spoken
          for. No kbd, because a binding is discovered in the palette and the
          tooltip, and this act is deliberately bound to no key. */}
      <Button variant="ghost" size="sm" ground="sunken" onClick={() => setOffered(compose())}>
        {FILE_AN_ISSUE}
      </Button>
      <FilingReview
        offered={offered}
        removed={removed}
        onToggle={(id) =>
          setRemoved((was) => {
            const next = new Set(was);
            if (next.has(id)) next.delete(id);
            else next.add(id);
            return next;
          })
        }
        onCancel={close}
        onCopy={(filing) => {
          copyIssue(filing, onCopied);
          close();
        }}
      />
    </>
  );
}

/** What the dialog asks, named once so nothing retypes it. */
export const REVIEW_TITLE = "Send this failure to an issue tracker?";

export type FilingReviewProps = {
  /** Everything offered, or `null` where nothing is under review. */
  offered: Filing | null;
  /** Which ids have been taken out. A required item is never in here. */
  removed: ReadonlySet<string>;
  onToggle: (id: string) => void;
  onCancel: () => void;
  /** Given what is actually going, which is `offered` minus what was removed. */
  onCopy: (filing: Filing) => void;
};

/**
 * The review itself — every row, what is not bounded about it, and its text.
 *
 * **Split from the control because the control is state and this is the
 * screen.** A story can render this open and show the thing the drawing is
 * about; a story of the button shows a button.
 */
export function FilingReview({ offered, removed, onToggle, onCancel, onCopy }: FilingReviewProps) {
  const filing =
    offered === null
      ? null
      : {
          ...offered,
          attached: offered.attached.filter(
            (item) => item.required === true || !removed.has(item.id),
          ),
        };

  return (
    <Dialog
      open={filing !== null}
      tone="neutral"
      width="wide"
      title={REVIEW_TITLE}
      confirmLabel={COPY_THE_ISSUE}
      confirmDisabled={filing === null || filing.attached.length === 0}
      onCancel={onCancel}
      onConfirm={() => {
        if (filing !== null) onCopy(filing);
      }}
    >
      {offered === null || filing === null ? null : (
        <div className="armada-filing">
          {/* What Armada did not do, before what it is about to hand over. The
              order matters: somebody who reads one sentence reads the one
              saying nothing was stripped. */}
          <p className="armada-filing__says">{NO_SCRUB}</p>
          <p className="armada-filing__says">{OPENS_NOTHING}</p>

          <ul className="armada-filing__rows">
            {offered.attached.map((item) => (
              <li className="armada-filing__row" key={item.id}>
                <div className="armada-filing__control">
                  {item.required === true ? (
                    // Not a checked, disabled checkbox: a control that cannot
                    // be operated reads as one that is broken, and this row is
                    // not a decision made for somebody. It is a fact about the
                    // artifact, stated after the name it is a fact about.
                    <>
                      <span className="armada-filing__label">{item.label}</span>
                      <span className="armada-filing__always">{ALWAYS}</span>
                    </>
                  ) : (
                    <Checkbox
                      checked={!removed.has(item.id)}
                      onChange={() => onToggle(item.id)}
                    >
                      {item.label}
                    </Checkbox>
                  )}
                </div>
                {/* The warning is the read-this mark, made specific. A row
                    whose text is unbounded says what is unbounded about it. */}
                <p className="armada-filing__warning">{item.warning}</p>
                <details className="armada-filing__read">
                  <summary className="armada-filing__summary">
                    <Fold />
                    {READ}
                  </summary>
                  {/* The exact text that will be in the body — not a second
                      rendering of the same facts. One producer, the rule the
                      payload's expanded view already rests on. */}
                  <pre className="armada-filing__text">{item.body}</pre>
                </details>
              </li>
            ))}
          </ul>

          {offered.withheld === undefined || offered.withheld.length === 0 ? null : (
            <div className="armada-filing__withheld">
              <p className="armada-filing__says">
                Not offered, and the issue body says so too — a reader who finds none of this
                cannot otherwise tell it was left out on purpose.
              </p>
              <ul className="armada-filing__rows">
                {offered.withheld.map((item) => (
                  <li className="armada-filing__row" key={item.label}>
                    <span className="armada-filing__label">{item.label}</span>
                    <p className="armada-filing__warning">{item.why}</p>
                  </li>
                ))}
              </ul>
            </div>
          )}

          <details className="armada-filing__read">
            <summary className="armada-filing__summary">
              <Fold />
              {`${READ} the whole body`}
            </summary>
            <pre className="armada-filing__text">{issueBody(filing)}</pre>
          </details>
        </div>
      )}
    </Dialog>
  );
}

/**
 * The fold's caret. `<details>` carries the keyboard and the accessible state,
 * and the registry pairs two glyphs for expand and collapse rather than
 * rotating one — so both are drawn and CSS shows the one that applies.
 */
function Fold() {
  return (
    <>
      <ChevronRight
        className="armada-filing__caret armada-filing__caret--shut"
        size={FOLD_ICON}
        strokeWidth={FOLD_STROKE}
        aria-hidden
      />
      <ChevronDown
        className="armada-filing__caret armada-filing__caret--open"
        size={FOLD_ICON}
        strokeWidth={FOLD_STROKE}
        aria-hidden
      />
    </>
  );
}
