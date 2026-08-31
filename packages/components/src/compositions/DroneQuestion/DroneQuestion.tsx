import type { ReactNode } from "react";
import { useState } from "react";

import { Button } from "../../primitives/Button/Button";
import { Radio, RadioGroup } from "../../primitives/Radio/Radio";

/**
 * A drone has asked a question and is waiting. The question, the answers it
 * offered, and the one control that sends one.
 *
 * **The whole surface is a closed set.** There is no text field here and no
 * prop that could add one: the drone offers between two and four answers, a
 * person picks one, and nothing is typed. That is the difference between this
 * and the orchestrator `docs/scope.md` records as abandoned — the distinction is
 * not whether a person is involved, it is whether a conversation is the medium.
 * A person who needs to say something the options do not cover has Redirect,
 * which already carries their own words into the same session; `redirectNote`
 * says so on this surface so that it is not a thing to remember.
 *
 * **Each answer says what it commits to, and that is not decoration.** A label
 * alone is a button whose effect has to be guessed, and here a guess produces
 * jobs that run and spend. The consequence sits under the label in the briefing
 * register the design contract asks for: the facts needed to decide, on screen,
 * without a click.
 *
 * **Nothing is preselected.** A default answer is an answer somebody did not
 * give, and the whole reason this exists is that the drone could not guess.
 *
 * # No glyph, and that is a finding rather than a choice
 *
 * `packages/icons/icons.toml` has no mark for a drone asking. `file-question-mark`
 * is reserved — "the file-* family means evidence throughout" — and nothing else
 * in the registry means it. So this draws none, per `armada-components`: a state
 * with no glyph gets the story and the report, never an invented mark.
 *
 * # It takes the waiting hue, and the job is `running`
 *
 * `--status-awaiting-review` is the "needs you, not urgent" token, which is the
 * condition on this surface exactly: nothing is wrong, and nothing moves until a
 * person acts. **The job's own badge is unaffected** — it says `running`, which
 * is true, because a question rides beside the state rather than being one.
 * Drawing this in the escalated hue would say something had gone wrong, and
 * drawing it in no hue at all would leave the one region on a running job's
 * screen that is waiting on a person looking like the three that are not.
 */
export type DroneQuestionProps = {
  /** What was asked, in the drone's own words. */
  question: string;
  /** The answers it will take. Two to four, each label distinct. */
  options: readonly DroneAnswer[];
  /** How long it has been waiting, already rendered. `12m`, `2h`. */
  waiting?: string;
  /** Send the label. The caller holds the request. */
  onAnswer: (label: string) => void;
  /**
   * An answer already in flight, or nothing live to send it over. The caller's
   * sentence says which — a disabled control with no reason looks broken.
   */
  disabled?: boolean;
  /** Why the controls are off, where they are. */
  disabledNote?: ReactNode;
  /** The line over the question. Sentence case, no Wh- opener. */
  label?: ReactNode;
  /** Where the words go when none of the answers is right. */
  redirectNote?: ReactNode;
  answerLabel?: string;
};

/** One answer, as this surface draws it. */
export type DroneAnswer = {
  /** What the person picks, and what the answer names. */
  label: string;
  /** What the drone will do if it is picked. Never blank. */
  consequence: string;
};

export function DroneQuestion({
  question,
  options,
  waiting,
  onAnswer,
  disabled = false,
  disabledNote,
  label = "The drone is waiting on you",
  redirectNote = "If none of these is right, redirect the drone instead — that is where your own words go.",
  answerLabel = "Send this answer",
}: DroneQuestionProps) {
  const [chosen, setChosen] = useState<string | null>(null);

  return (
    <section className="armada-question" aria-label="A question from the drone">
      <div className="armada-question__head">
        <span className="armada-question__label">{label}</span>
        {/* Aged by the caller and never here. The instant crosses once and
            nothing on the wire ticks, so a surface that formatted its own
            elapsed would be a second reading of one fact. */}
        {waiting === undefined ? null : (
          <span className="armada-question__waiting mono">{waiting}</span>
        )}
      </div>

      {/* The drone's own sentence, quoted rather than framed. Fleet adds no
          wording to it and neither does this. */}
      <p className="armada-question__asked">{question}</p>

      <RadioGroup label="Answers the drone offered">
        {options.map((option) => (
          <div className="armada-question__option" key={option.label}>
            <Radio
              name="armada-question"
              value={option.label}
              checked={chosen === option.label}
              disabled={disabled}
              onChange={() => setChosen(option.label)}
            >
              {option.label}
            </Radio>
            {/* Under the label rather than beside it: this is what the choice
                commits to, and a person reads it after the name and before the
                press. */}
            <p className="armada-question__means">{option.consequence}</p>
          </div>
        ))}
      </RadioGroup>

      {/* Off until something is picked, which is what fleet would answer — a
          round trip to learn nothing was chosen is a refusal that reads as a
          failure. */}
      <Button
        variant="primary"
        disabled={disabled || chosen === null}
        onClick={() => chosen !== null && onAnswer(chosen)}
      >
        {answerLabel}
      </Button>

      <p className="armada-question__said">{redirectNote}</p>
      {disabled && disabledNote !== undefined ? (
        <p className="armada-question__said" role="note">
          {disabledNote}
        </p>
      ) : null}
    </section>
  );
}
