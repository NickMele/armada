import type { ReactNode } from "react";
import { useId, useState } from "react";

import { Button } from "../../primitives/Button/Button";
import { Prose } from "../../primitives/Prose/Prose";
import { Radio, RadioGroup } from "../../primitives/Radio/Radio";
import { Textarea } from "../../primitives/Textarea/Textarea";
import type { GamingFlagAt } from "../GamingFlags/GamingFlags";
import { UnifiedDiff, type DiffFile } from "../UnifiedDiff/UnifiedDiff";

/**
 * One flag holding a step, as a person has to read it to decide. #1079.
 *
 * **The headline and explanation are the registry's**, handed down by the
 * caller. They are copy about an enum, and written here they would be a
 * second vocabulary that says one pattern two ways.
 */
export type HeldFinding = {
  /** The wire spelling. Drawn in mono only where no headline arrived. */
  pattern: string;
  /** `A test may have been weakened to make this step pass`. */
  headline?: string;
  /** What the flag means and why a person has to look, in a sentence or two. */
  explanation?: string;
  /**
   * The flagged lines, as the hunk of the patch that holds them. **Located by
   * the caller from the patch, never from an invented line** — a flag on a
   * removed line has no post-image number, and is found by what it quotes.
   * Absent draws the citation and the location instead.
   */
  hunk?: DiffFile;
  /** What the check quoted, drawn where no hunk was located. */
  cited?: string;
  /** Where it is, drawn where no hunk was located. */
  at?: GamingFlagAt;
  /** The question the check answered. Absent on the three the diff decides. */
  asked?: string;
  /** The brief the question was asked in, by its path. */
  brief?: string;
};

/** What one answer does, and why it is off where it is. */
export type HeldAnswer = {
  /** What pressing it does, in a sentence, under the answer. */
  consequence: ReactNode;
  /** Why this answer cannot be taken here. Present turns its control off. */
  withheld?: ReactNode;
};

export type HeldFlagProps = {
  /** Every flag holding the step, in the order the check answered. Empty draws nothing. */
  findings: HeldFinding[];
  onOpenBrief?: (brief: string) => void;
  /** *No, the work is fine*. The reason is optional: a preset, a note, both, or nothing. */
  carryOn: HeldAnswer & { presets: readonly string[]; onCarryOn: (reason: string) => void };
  /** *Yes, the test was weakened*. The note is optional and goes to the Drone. */
  sendBack: HeldAnswer & { onSendBack: (note?: string) => void };
  /** An act on this Job is already out, or what is shown is not live. */
  disabled?: boolean;
  /** Why both answers are off, where they are. */
  disabledNote?: ReactNode;
};

const IT_ASKED = "It asked";
const OPEN_THE_BRIEF = "Open the brief";
const IS_IT_RIGHT = "Is the flag right?";
const NO_IT_IS_FINE = "No, the work is fine";
const CARRY_ON = "Carry on";
const YES_IT_WAS = "Yes, the test was weakened";
const SEND_IT_BACK = "Send it back";
const REASON = "Reason (optional)";
const NOTE = "Note (optional)";
const NOTE_FOR_THE_DRONE = "Note for the drone (optional)";

/**
 * The card a gaming flag holds a step with: what it means, the lines it is
 * about, what it asked, and the two answers a person can give.
 *
 * **Both answers, with what each does, and neither needs typing.** A reason
 * is asked for and never required, so *Carry on* and *Send it back* are each
 * one press — the definition of done in #1079.
 *
 * **Secondary, both.** Neither is an approval: carrying on says the check was
 * wrong, and sending it back says the Drone was. A fill on either would say
 * which the screen expects.
 */
export function HeldFlag({
  findings,
  onOpenBrief,
  carryOn,
  sendBack,
  disabled = false,
  disabledNote,
}: HeldFlagProps) {
  const [preset, setPreset] = useState<string | null>(null);
  const [reasonNote, setReasonNote] = useState("");
  const [droneNote, setDroneNote] = useState("");
  const group = useId();
  if (findings.length === 0) return null;

  function carry() {
    // The preset text, the note, both, or nothing — Fleet takes a blank reason
    // on a gaming flag, and the words it does get are the person's own.
    const said = [preset ?? "", reasonNote.trim()].filter((part) => part !== "").join(". ");
    carryOn.onCarryOn(said);
  }

  function sendIt() {
    const said = droneNote.trim();
    sendBack.onSendBack(said === "" ? undefined : said);
  }

  return (
    <div className="armada-held-flag">
      {findings.map((finding, at) => (
        <article
          className="armada-held-flag__finding"
          key={`finding-${at}`}
          aria-label={finding.headline ?? finding.pattern}
        >
          {finding.headline === undefined ? (
            <span className="armada-held-flag__pattern">{finding.pattern}</span>
          ) : (
            <p className="armada-held-flag__headline">{finding.headline}</p>
          )}
          {finding.explanation === undefined ? null : (
            <p className="armada-held-flag__explanation">{finding.explanation}</p>
          )}
          {finding.hunk !== undefined ? (
            <UnifiedDiff files={[finding.hunk]} emptyNote="" />
          ) : (
            <>
              {finding.cited === undefined || finding.cited === "" ? null : (
                <div className="armada-held-flag__cited">
                  <Prose text={finding.cited} />
                </div>
              )}
              {finding.at === undefined ? null : (
                <span className="armada-held-flag__at">
                  {finding.at.line === undefined ? finding.at.file : `${finding.at.file}:${finding.at.line}`}
                </span>
              )}
            </>
          )}
          {finding.asked === undefined ? null : (
            <div className="armada-held-flag__asked">
              <span className="armada-held-flag__label">{IT_ASKED}</span>
              <p className="armada-held-flag__question">{finding.asked}</p>
              {finding.brief === undefined || onOpenBrief === undefined ? null : (
                <Button variant="ghost" size="sm" onClick={() => onOpenBrief(finding.brief as string)}>
                  {OPEN_THE_BRIEF}
                </Button>
              )}
            </div>
          )}
        </article>
      ))}

      <div className="armada-held-flag__decide" role="group" aria-label={IS_IT_RIGHT}>
        <p className="armada-held-flag__headline">{IS_IT_RIGHT}</p>

        <div className="armada-held-flag__answer">
          <span className="armada-held-flag__answer-label">{NO_IT_IS_FINE}</span>
          <p className="armada-held-flag__means">{carryOn.consequence}</p>
          {carryOn.presets.length === 0 ? null : (
            <RadioGroup label={REASON}>
              {carryOn.presets.map((one) => (
                <Radio
                  key={one}
                  name={`${group}-reason`}
                  value={one}
                  checked={preset === one}
                  disabled={disabled || carryOn.withheld !== undefined}
                  onChange={() => setPreset(one)}
                >
                  {one}
                </Radio>
              ))}
            </RadioGroup>
          )}
          <Textarea
            label={NOTE}
            rows={2}
            value={reasonNote}
            disabled={disabled || carryOn.withheld !== undefined}
            onChange={(event) => setReasonNote(event.target.value)}
          />
          <div className="armada-held-flag__press">
            <Button
              variant="secondary"
              disabled={disabled || carryOn.withheld !== undefined}
              onClick={carry}
            >
              {CARRY_ON}
            </Button>
          </div>
          {carryOn.withheld === undefined ? null : (
            <p className="armada-held-flag__withheld" role="note">
              {carryOn.withheld}
            </p>
          )}
        </div>

        <div className="armada-held-flag__answer">
          <span className="armada-held-flag__answer-label">{YES_IT_WAS}</span>
          <p className="armada-held-flag__means">{sendBack.consequence}</p>
          <Textarea
            label={NOTE_FOR_THE_DRONE}
            rows={2}
            value={droneNote}
            disabled={disabled || sendBack.withheld !== undefined}
            onChange={(event) => setDroneNote(event.target.value)}
          />
          <div className="armada-held-flag__press">
            <Button
              variant="secondary"
              disabled={disabled || sendBack.withheld !== undefined}
              onClick={sendIt}
            >
              {SEND_IT_BACK}
            </Button>
          </div>
          {sendBack.withheld === undefined ? null : (
            <p className="armada-held-flag__withheld" role="note">
              {sendBack.withheld}
            </p>
          )}
        </div>

        {disabled && disabledNote !== undefined ? (
          <p className="armada-held-flag__withheld" role="note">
            {disabledNote}
          </p>
        ) : null}
      </div>
    </div>
  );
}
