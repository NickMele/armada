import { useEffect, useId, useState } from "react";

import type { JudgeAnswer } from "@armada/protocol";
import { JUDGE_FINDING, JUDGE_FINDING_LABEL, JUDGE_FINDING_SAID } from "../../judge-record";
import { Button, STILL_WAITING, useStillWaiting } from "../../primitives/Button/Button";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/** What each answer's button says while it is out and Fleet has not answered. #1117. */
const UNDERWAY: Record<JudgeAnswer, string> = {
  agree: "Agreeing…",
  disagree_once: "Disagreeing, just this step…",
  disagree_always: "Standing this criterion down…",
};

/**
 * The Judge's three answers, named once. **The one source** — the dock draws
 * the same three from every repository's questions (#938), and used to carry
 * its own copy of the labels below.
 */
export const JUDGE_ANSWER: Record<JudgeAnswer, { label: string; means: string }> = {
  agree: {
    label: "Agree with the refusal",
    means: "The step fails, as it would where the criterion is marked refuse.",
  },
  disagree_once: {
    label: "Disagree, just this step",
    means: "The step advances. The next job's gate asks about this criterion again.",
  },
  disagree_always: {
    label: "Always disagree",
    means: "The step advances, and no later job in this repository is asked about this criterion.",
  },
};

/**
 * A Judge criterion refused and a person is being asked about it, rather than
 * the step stopping over it — `docs/concepts/judge.md`'s asking design.
 *
 * **Drawn where the verdict is drawn now.** A refusal on a criterion marked
 * `refuse` still stops the step exactly as it always has; this is what the
 * rest of them do instead, and it carries the same finding a stopped step's
 * refusal does — `expected`, `produced`, `consequence` — because a person
 * answering this needs exactly what a person overruling one needs.
 *
 * **One press is the whole answer.** There is no radio-then-send here, unlike
 * `DroneQuestion`: three presses, three outcomes, and a note that rides along
 * without gating any of them. Agreeing fails the step exactly as it would if
 * the criterion were marked `refuse`; either disagreement advances it, and
 * "always" also stands the criterion down for the repository.
 *
 * **Nothing here times out.** An unanswered question holds — the same cost an
 * open human gate already has — so there is no countdown and no default.
 *
 * **One primary, and it is the one-step answer.** "Always disagree" changes
 * what this repository asks about forever, so it must not read as the
 * inviting press — it is secondary, beside "Agree with the refusal", and the
 * emphasis sits on "Disagree, just this step" alone.
 */
export type JudgeQuestionProps = {
  /** Which criterion refused — `Does the fix address the cause the note names?` */
  question: string;
  /** What should be seen, returned or recorded if the work is right. */
  expected: string;
  /** What will be seen instead. */
  produced: string;
  /** What that difference does to whoever consumes it — the field to triage on. */
  consequence: string;
  /** Send the answer, with the note as typed — never required. */
  onAnswer: (answer: "agree" | "disagree_once" | "disagree_always", note?: string) => void;
  /** An answer already in flight, or nothing live to send it over. */
  disabled?: boolean;
  /** Why the controls are off, where they are. */
  disabledNote?: string;
  /**
   * The answer that was pressed and Fleet has not answered. That control waits
   * and every other one is off; `false` is nothing out. #1117.
   */
  pending?: boolean;
};

export function JudgeQuestion({
  question,
  expected,
  produced,
  consequence,
  onAnswer,
  disabled = false,
  disabledNote,
  pending = false,
}: JudgeQuestionProps) {
  const [note, setNote] = useState("");
  const noteId = useId();
  const finding = { expected, produced, consequence };
  // Which of the three was pressed. Local, and cleared once `pending` clears —
  // the caller only ever says an answer from this block is out, not which.
  const [pressed, setPressed] = useState<JudgeAnswer | null>(null);
  useEffect(() => {
    if (!pending) setPressed(null);
  }, [pending]);
  const stillWaiting = useStillWaiting(pending);
  const off = disabled || pending;

  const send = (answer: "agree" | "disagree_once" | "disagree_always") => {
    setPressed(answer);
    const trimmed = note.trim();
    onAnswer(answer, trimmed === "" ? undefined : trimmed);
  };

  return (
    <section className="armada-judge-question" aria-label="A judge refusal you are being asked about">
      <p className="armada-judge-question__asked">{question}</p>

      <dl className="armada-judge-question__finding">
        {JUDGE_FINDING.map((field) => (
          <div className="armada-judge-question__field" key={field}>
            <Tooltip asChild label={JUDGE_FINDING_SAID[field]}>
              <dt className="armada-judge-question__label">{JUDGE_FINDING_LABEL[field]}</dt>
            </Tooltip>
            <dd className="armada-judge-question__value" data-field={field}>
              {finding[field]}
            </dd>
          </div>
        ))}
      </dl>

      <Textarea
        id={noteId}
        label="Note (optional)"
        rows={2}
        value={note}
        disabled={off}
        onChange={(event) => setNote(event.target.value)}
      />

      <div className="armada-judge-question__answers" role="group" aria-label="Your answer">
        <Button
          variant="secondary"
          pending={pressed === "agree" && pending}
          disabled={off}
          onClick={() => send("agree")}
        >
          {pressed === "agree" && pending ? UNDERWAY.agree : JUDGE_ANSWER.agree.label}
        </Button>
        <Button
          variant="primary"
          pending={pressed === "disagree_once" && pending}
          disabled={off}
          onClick={() => send("disagree_once")}
        >
          {pressed === "disagree_once" && pending ? UNDERWAY.disagree_once : JUDGE_ANSWER.disagree_once.label}
        </Button>
        <Button
          variant="secondary"
          pending={pressed === "disagree_always" && pending}
          disabled={off}
          onClick={() => send("disagree_always")}
        >
          {pressed === "disagree_always" && pending
            ? UNDERWAY.disagree_always
            : JUDGE_ANSWER.disagree_always.label}
        </Button>
      </div>

      {stillWaiting ? (
        <p className="armada-judge-question__said" role="status">
          {STILL_WAITING}
        </p>
      ) : disabled && disabledNote !== undefined ? (
        <p className="armada-judge-question__said" role="note">
          {disabledNote}
        </p>
      ) : null}
    </section>
  );
}
