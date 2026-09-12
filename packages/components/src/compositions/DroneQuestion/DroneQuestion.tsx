import type { ReactNode } from "react";
import { useId, useState } from "react";

import { Button } from "../../primitives/Button/Button";
import { Radio, RadioGroup } from "../../primitives/Radio/Radio";
import { Textarea } from "../../primitives/Textarea/Textarea";

/**
 * A drone has asked a question and is waiting, or a command it reached for and
 * was not given. The question, the answers offered, and the control that sends
 * one.
 *
 * **No frame and no fill of its own**, only the waiting rule on its left — the
 * band above carries the condition, and two framed regions read as two faults.
 *
 * **Nothing is preselected**, and each answer says what it commits to: a label
 * alone is a button whose effect has to be guessed, and a guess here spends.
 *
 * **Still not the orchestrator `docs/scope.md` records as abandoned**: the
 * answers stay a closed set and no reply comes back — see [`DroneAnswer`]. No
 * glyph either: `icons.toml` has none for this, and the gap is reported.
 */
export type DroneQuestionProps = {
  /**
   * What was asked: the drone's own words, or the command it wants to run and
   * was not given. **A node rather than a string** so a command can sit in
   * mono inside the sentence around it — it is what the drone sent, and mono
   * is how a surface says a value is the machine's.
   */
  question: ReactNode;
  /** The answers it will take. Two to four, each label distinct. */
  options: readonly DroneAnswer[];
  /** How long it has been waiting, already rendered. `12m`, `2h`. */
  waiting?: string;
  /**
   * Send the label, and the words typed under it where that answer takes any.
   *
   * **An answer carrying no words calls this with one argument**, as it did
   * before a note existed — not with an explicit `undefined`. A caller that
   * offers no note cannot tell the two versions apart.
   */
  onAnswer: (label: string, note?: string) => void;
  /**
   * An answer already in flight, or nothing live to send it over. The caller's
   * sentence says which — a disabled control with no reason looks broken.
   */
  disabled?: boolean;
  /** Why the controls are off, where they are. */
  disabledNote?: ReactNode;
  /** The line over the question. Sentence case, no Wh- opener. */
  label?: ReactNode;
  /**
   * The sentence under the control: where the words go when none of the
   * answers is right, or — where the answers are the whole set, as they are
   * for a command — what an answer given late still does.
   */
  redirectNote?: ReactNode;
  answerLabel?: string;
  /**
   * What the answers are, said over them. **Whose offer it is**: a drone's
   * question offers its own answers, and a command's three are Armada's — a
   * heading crediting the drone would say it chose them.
   */
  answersLabel?: string;
  /**
   * A reading of what was asked, as the caller holds it. **Absent draws no
   * control**, which is a drone's own question: there is no command to read,
   * and offering to explain a sentence written in English answers nothing.
   */
  explain?: Explaining;
  /** Ask for the reading. The caller makes the request and reports it back on `explain`. */
  onExplain?: () => void;
  /** The words on the control. Short enough to press without reading twice. */
  explainLabel?: string;
};

/**
 * One answer, as this surface draws it.
 *
 * **`noteLabel` is what makes an answer the one that carries words**, and the
 * caller says which. For a drone's own question nothing is typed and the closed
 * set holds — Redirect is where those words go. A command is the other case:
 * its three answers are Armada's rather than the drone's, and only one wants a
 * reason, since an allow tells the drone everything it acts on and a refusal
 * tells it nothing. That reason is in a person's head as they press reject, and
 * a second box spends it. Since protocol 11.5.
 */
export type DroneAnswer = {
  /** What the person picks, and what the answer names. */
  label: string;
  /** What the drone will do if it is picked. Never blank. */
  consequence: string;
  /**
   * That this answer carries a person's words, and what the field is called.
   * **Drawn only while this answer is the chosen one** — a field under an
   * answer nobody picked is a question nobody asked.
   */
  noteLabel?: string;
  /** What becomes of those words, under the field. Absent draws nothing. */
  noteSays?: ReactNode;
};

/**
 * A reading of the command, as the caller holds it.
 *
 * **It decides nothing.** A person cannot answer about a command they cannot
 * read, and the alternative is leaving the window — which is how a command gets
 * allowed unread. The answers stay live while it is out and after it lands.
 *
 * **The model is named because a reading is a claim**, which is what
 * hedge-by-source asks of anything judged rather than measured.
 */
export type Explaining =
  /** Nothing asked yet, which is every command until somebody presses. */
  | { state: "ready" }
  /** Asked, nothing back. The control says so and does not send twice. */
  | { state: "asking" }
  /** What came back, and what read it. */
  | { state: "read"; explanation: string; model: string }
  /** Nothing came back, in the caller's words. The answers are untouched. */
  | { state: "failed"; why: ReactNode };

/** What the reading is called where a screen reader meets it. */
const READING_LABEL = "What this command does";

/** Said while the reading is out: a wait somebody asked for, named rather than spun. */
const READING = "Reading this command.";

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
  answersLabel = "Answers the drone offered",
  explain,
  onExplain,
  explainLabel = "Help me understand this command",
}: DroneQuestionProps) {
  const [chosen, setChosen] = useState<string | null>(null);
  // Kept when the choice moves off the answer that reads it: a person who typed
  // a reason and then pressed the wrong radio has not withdrawn the reason.
  const [note, setNote] = useState("");
  // One radio group per box. A drone's question and a command it is waiting
  // on can be open at once, and two boxes sharing a name are one group to the
  // browser: picking in one would clear the other.
  const group = useId();

  const picked = options.find((option) => option.label === chosen);

  function send() {
    if (chosen === null) return;
    // An answer that reads no words is sent as it was before one could be
    // typed — one argument, and no empty string standing in for silence.
    if (picked?.noteLabel === undefined) {
      onAnswer(chosen);
      return;
    }
    const said = note.trim();
    onAnswer(chosen, said === "" ? undefined : said);
  }

  // The job stays `running` and its badge is right to say so: a question rides
  // beside the state rather than being one, so nothing in the header moves
  // while this is open. `--step-waiting` is the tone, which the screen's own
  // waiting notice takes and which means "needs you, not urgent".
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

      {/* Above the answers and never among them: reading is what a person does
          before deciding, and a control in the list would read as a fourth
          answer. */}
      {explain === undefined ? null : (
        <div className="armada-question__explaining">
          <Button
            variant="secondary"
            onClick={onExplain}
            // Off only while its own reading is out. **Never `disabled`** — a
            // command whose answer is on its way is still one somebody may want
            // to understand, and this sends nothing.
            disabled={explain.state === "asking"}
            aria-busy={explain.state === "asking" || undefined}
          >
            {explainLabel}
          </Button>
          {explain.state === "asking" ? (
            <p className="armada-question__said" role="status">
              {READING}
            </p>
          ) : null}
          {explain.state === "read" ? (
            <div className="armada-question__reading" role="status" aria-label={READING_LABEL}>
              <p className="armada-question__explanation">{explain.explanation}</p>
              {/* Mono on the model, as every machine-derived value here is. */}
              <p className="armada-question__said">
                Read by <span className="mono">{explain.model}</span>
              </p>
            </div>
          ) : null}
          {explain.state === "failed" ? (
            <p className="armada-question__said" role="status">
              {explain.why}
            </p>
          ) : null}
        </div>
      )}

      <RadioGroup label={answersLabel}>
        {options.map((option) => (
          <div className="armada-question__option" key={option.label}>
            <Radio
              name={group}
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
            {/* The field the answer reads, under the answer that reads it, so
                nothing asks for words about a decision nobody has taken. */}
            {option.noteLabel !== undefined && chosen === option.label ? (
              <div className="armada-question__note">
                <Textarea
                  label={option.noteLabel}
                  rows={2}
                  value={note}
                  disabled={disabled}
                  onChange={(event) => setNote(event.target.value)}
                />
                {option.noteSays === undefined ? null : (
                  <p className="armada-question__means">{option.noteSays}</p>
                )}
              </div>
            ) : null}
          </div>
        ))}
      </RadioGroup>

      {/* Off until something is picked, which is what fleet would answer — a
          round trip to learn nothing was chosen is a refusal that reads as a
          failure. */}
      <Button variant="primary" disabled={disabled || chosen === null} onClick={send}>
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
