/**
 * The issue body an error becomes, and the review that composes it.
 *
 * **Copying stays on the machine. Filing leaves it.** `copyDebugInfo` puts one
 * artifact on the clipboard for a person to read, paste into a terminal or
 * send to somebody they already trust. This composes something different: the
 * thing a person pastes into a tracker, where it is public and permanent. The
 * two acts differ in where they end, so they differ in what stands between the
 * press and the artifact — nothing, and a review.
 *
 * # Nothing here sends
 *
 * **There is no transport, and this does not pretend to have one.** Fleet holds
 * no credential for an issue tracker and nothing on the wire names the
 * repository's remote — the same wall `renderer/src/Report.tsx` met when it
 * filed a report about a Job, and it answered it the same way: produce the body,
 * put it on the clipboard, and say plainly that Armada opened nothing.
 *
 * That is why there is no **Send**, and why the drawing's Reported strip is not
 * built. A strip carrying the issue link, the time and the count needs an issue
 * number, and no issue number ever comes back to Bridge. So does offering an
 * already-filed issue to the second occurrence of a code. Neither is written
 * here as a blank waiting to be filled in.
 *
 * # What the body says about itself
 *
 * **The body carries what was attached and what was not, by name.** The drawing
 * wanted the count on a strip, on the grounds that somebody coming back needs
 * to know whether the transcript went and the issue body is the only other
 * place that answer lives. With no strip it is the only place, and a list of
 * names answers the question a count only gestures at.
 *
 * # Fences, here and not in the payload
 *
 * `payload.ts` refuses fences because its artifact has to survive an issue
 * body, a chat message and a terminal scrollback, and a fence helps in one of
 * the three. This artifact has one destination, and it is the one where a
 * fence is the difference between aligned columns and a reflowed paragraph.
 * So this fences, and sizes the fence to the content rather than assuming
 * three backticks: an error's `message` is prose a Rust `Display` impl wrote,
 * and one containing a fence would otherwise break the block silently.
 */

import { debugInfo, SAFETY } from "../ErrorNotice/payload";
import type { DebugPayload } from "../ErrorNotice/payload";

/** The shortest fence Markdown takes. */
const MIN_FENCE = 3;

/**
 * One thing a person is deciding whether to send.
 *
 * **Every item carries its own warning, and the warning is the read-this
 * mark.** The drawing marked two rows "read this" and left the envelope
 * unmarked on the grounds that it was bounded. It is not — `message` and
 * `chain` are prose — so a generic mark would either be missing from the one
 * row that needs it or be on every row and say nothing. A sentence naming what
 * this particular item is not bounded by says the thing the mark was standing
 * in for.
 */
export type Attachment = {
  /** Stable, so a row can be keyed and toggled. */
  id: string;
  /** What the item is. Sentence case, no trailing colon. */
  label: string;
  /**
   * What is not bounded about this item, in one sentence. **Never a
   * reassurance** — Armada makes no scrub claim, and a row with nothing to
   * warn about would be a row nobody had thought about.
   */
  warning: string;
  /** The text itself, exactly as it will appear in the body. */
  body: string;
  /**
   * Whether it can be taken out.
   *
   * **True of the envelope and nothing else, and it is not a safety claim.**
   * An error with its own record removed is a sentence somebody typed, which
   * is the thing this replaces. It is required because it is the report, not
   * because it is bounded — which is why it carries the same warning every
   * other row does rather than being waved through as structured and safe.
   */
  required?: boolean;
};

/**
 * Something a reader might expect and will not find.
 *
 * **Named in the body, not silently absent.** A maintainer reading an Armada
 * issue with no transcript in it cannot otherwise tell whether one was withheld
 * on purpose, lost, or never existed — and the first of the three is a
 * different conversation from the other two.
 */
export type Withheld = {
  label: string;
  /** Why it is not here. A fact about Armada, never an apology. */
  why: string;
};

/** A fence long enough that nothing inside the block can close it early. */
function fence(body: string): string {
  let longest = 0;
  let run = 0;
  for (const character of body) {
    run = character === "`" ? run + 1 : 0;
    if (run > longest) longest = run;
  }
  return "`".repeat(Math.max(MIN_FENCE, longest + 1));
}

/**
 * What the body says about scrubbing, before anything it carries.
 *
 * **It states what Armada does not do.** A promise to strip secrets is one
 * nothing here could keep — the payload's prose fields are written by whatever
 * raised the error, and the items beside them are records of what a machine did
 * on somebody's own disk. A claim it cannot keep is worse than the work of
 * reading, which is why the dialog puts the text of every item on screen and
 * this says out loud that no pass was made over it.
 */
export const NO_SCRUB =
  "Nothing here was scrubbed. Armada removes nothing on the way out, so what is below is what left the machine.";

export type Filing = {
  /** The heading. The error's own words, which is what a reader greps for. */
  title: string;
  /** What is going, in the order it will be read. */
  attached: readonly Attachment[];
  /** What is not, and why. May be empty. */
  withheld?: readonly Withheld[];
};

/**
 * The issue body, as the text that goes on the clipboard.
 *
 * Order is fixed and is the order it is read: the heading, what Armada does not
 * claim about any of it, each attached item under its own name, then what was
 * left out. A caller cannot reorder it, for the reason `debugInfo` gives — two
 * bodies that differ in section order are two artifacts nobody can diff.
 */
export function issueBody(filing: Filing): string {
  const lines: string[] = [`# ${filing.title}`, "", NO_SCRUB];

  for (const item of filing.attached) {
    const wrap = fence(item.body);
    lines.push("", `## ${item.label}`, "", item.warning, "", wrap, item.body, wrap);
  }

  const withheld = filing.withheld ?? [];
  if (withheld.length > 0) {
    lines.push("", "## Not attached", "");
    for (const item of withheld) lines.push(`- **${item.label}** — ${item.why}`);
  }

  return lines.join("\n");
}

/**
 * Put a composed issue body on the clipboard, and say so either way.
 *
 * The same shape as `copyDebugInfo` and for the same reason: a clipboard write
 * is silent by nature, and a failed one is indistinguishable from a dead
 * control, so the surface is told on both paths and raises the same toast.
 */
export function copyIssue(filing: Filing, onCopied?: (what: string) => void): void {
  const said = () => onCopied?.(COPIED_ISSUE);
  void navigator.clipboard.writeText(issueBody(filing)).then(said, said);
}

/** What the toast says was copied. A noun, like `COPIED`: it is a body, not a value. */
export const COPIED_ISSUE = "The issue";

/**
 * What the act is called, on the control that opens the review.
 *
 * **It names the review, not the send**, because opening this is all the
 * control does — the artifact is composed on the second press and copied on
 * the way out. It is bound to no key. `c` is `copy debug info` and stays that;
 * a key that put a public artifact one keystroke from an error would undo the
 * one rule this whole flow exists for.
 */
export const FILE_AN_ISSUE = "File an issue";

/** What the dialog's confirm says, because it is what the press does. */
export const COPY_THE_ISSUE = "Copy the issue";

/**
 * What the dialog says Armada did not do.
 *
 * Verbatim in spirit with the report dialog's, which met the same wall on the
 * same evidence. A control that said "file the issue" and filed nothing would
 * be worse than one that says what it did.
 */
export const OPENS_NOTHING =
  "Armada does not open anything in the tracker. Copying puts the issue on your clipboard; the last step is yours.";

/** What the error's own record is called on its row and in the body. */
const ENVELOPE = "The error's own record";

/**
 * The payload, as the one item every filing carries.
 *
 * **Required, and that is not a claim about what it holds.** The drawing locked
 * it on because it is "structured and bounded; cannot carry a credential", and
 * that is true of `fields` and false of the artifact — `message` and `chain`
 * are prose written by whatever error `Display` impl raised them. It is
 * required because an issue with the record taken out is a sentence somebody
 * typed, which is the thing this replaces.
 *
 * So it carries `SAFETY`, the payload's own sentence pair, rather than a claim
 * written for this dialog. One mechanism, one statement of it — and the row a
 * person cannot remove is the row whose warning had better be the true one.
 */
export function envelopeOf(payload: DebugPayload): Attachment {
  return {
    id: "envelope",
    label: ENVELOPE,
    warning: SAFETY,
    body: debugInfo(payload),
    required: true,
  };
}

/**
 * What a reader might expect in an Armada issue and will not find.
 *
 * **The transcript, and it is the only one said on screen.** It is the item
 * somebody looks for, and it is the only one of the drawing's five whose
 * absence is a decision rather than a gap: `[observe-transcript-sharing]` on
 * `docs/concepts/observe.md` asks whether an observed transcript may leave the
 * machine at all, names attaching one to a bug report as removing today's
 * bound, and has no answer. **A row here default-on would answer it by
 * shipping**, which is not a thing built in the course of something else.
 *
 * The other three the drawing named are absent for a duller reason and are
 * recorded in `docs/contracts/error-contract.md` rather than on screen: doctor
 * is not built, and a judge response and a diff belong to a Job read whole,
 * which no failure surface holds.
 */
export const NOT_OFFERED: readonly Withheld[] = [
  {
    label: "The drone's turns",
    why: "Whether a transcript may leave this machine is not decided. It carries every command the drone ran and every path it touched.",
  },
];
