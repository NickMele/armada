import {
  Check,
  CircleCheck,
  CircleDot,
  CircleMinus,
  CircleX,
  Eye,
  ShieldCheck,
  ShieldMinus,
  ShieldX,
  UserCheck,
  X,
  type LucideIcon,
} from "lucide-react";
import type { ReactNode } from "react";

/**
 * What a stage of the phase strip opens to — what that stage is, what it is
 * waiting on, and where it stands.
 *
 * **The explanation lives where the question is asked.** That is the whole
 * argument for this component: a person looking at `build, test` and wondering
 * what a Check is should not have to remember a page that says so. Opening the
 * stage says it, beside the thing that prompted the question.
 *
 * **Checks and the Judge are different in kind, and this is where that shows.**
 * A Check is a command the repository declares and Fleet runs, judged by an
 * exit code, and it may pass or fail. The Judge is a model reading the work
 * against the step's acceptance criteria, and it may only refuse. Drawing them
 * as one row of chips risks reading as one kind of thing, so each carries a
 * standing sentence written once, here — `SAID` below — rather than retyped on
 * every screen that draws a strip.
 *
 * **The standing sentence does not replace this step's facts.** It says what
 * the tier is; the rows say what happened. A card that showed only the
 * definition is the defect a Judge refusal had: two criterion ids, two
 * verdicts, and a paragraph about what a Judge is — a person reading it learnt
 * nothing about their own Job.
 *
 * **Two shapes, one content.** `floating` is the card as it opens off the
 * strip: `--bg-overlay`, the strong edge, and the one shadow the contract
 * allows a floating layer. Without it the same card sits flat on the panel,
 * which is how the three are drawn side by side.
 */

/**
 * What a stage is. The three phases share one kind because nothing standing is
 * true of one that is not true of the others; the three tiers each have their
 * own, because what each tier is is exactly what a reader opens it to learn.
 */
export type PhaseStageKind = "phase" | "checks" | "judge" | "human";

/**
 * Where a stage stands. **Not a step activity and not a Job status** — those
 * belong to the tree and the badge. Hue comes from the below-Job-level tokens
 * `packages/tokens/src/status.css` declares and nowhere else: cleared is
 * `--step-advanced`, current is `--step-running`, waiting is `--step-waiting`,
 * failed is `--step-failed`, and a stage still ahead takes no hue at all.
 *
 * **`never` is a tier this step can never reach, and it is not `ahead`.** A
 * step whose `advance_gate` is `auto` will never stop for a person, and drawing
 * that as `ahead` made it identical to the tier of a step that *will* stop and
 * has not got there yet — two different facts, one chip. It takes no hue
 * either, because hue below Job level exists only where `tokens/status.css`
 * declares it and amber is already spent on waiting on you; the pair is told
 * apart by label and glyph, the way every same-hue state here is.
 */
export type PhaseStageState = "cleared" | "current" | "waiting" | "failed" | "ahead" | "never";

/**
 * The glyph a stage carries, keyed by what it is and where it stands.
 *
 * **The family says what the stage is and the member says where it stands.**
 * That split is the icon registry's, not this file's: `shield-*` means gates
 * and Checks throughout, `circle-*` is reserved to Judge verdicts, `user-check`
 * is the only human silhouette in the set, and the three phases borrow the
 * step-activity marks under the registry's borrowing convention.
 *
 * **A Check in flight takes `shield-minus`.** The drawing gives it a bare
 * shield outline and the registry has none — `shield-minus` is the nearest
 * declared member, reserved to Check results and meaning not reached, which is
 * true of a command still running. Reported.
 *
 * A phase still ahead carries no glyph. It is a position, not a state, and
 * there is nothing to depict.
 *
 * **A `never` tier carries none either, and for the human tier that is the
 * registry's rule rather than a choice.** `user-check` is reserved to *human
 * required, or actor=human*, and a step whose gate never asks for a person
 * requires none — so the one human silhouette in the set may not be drawn
 * there. Its absence beside the changed label is what tells the never-asks
 * tier from a tier not yet reached. Only the human tier can hold `never`; the
 * other three declare it because the record is total, and draw nothing rather
 * than borrow a mark that means something else.
 */
const GLYPHS: Record<PhaseStageKind, Record<PhaseStageState, LucideIcon | undefined>> = {
  phase: {
    ahead: undefined,
    never: undefined,
    current: CircleDot,
    cleared: Check,
    waiting: Eye,
    failed: X,
  },
  checks: {
    ahead: ShieldMinus,
    never: undefined,
    current: ShieldMinus,
    cleared: ShieldCheck,
    waiting: ShieldMinus,
    failed: ShieldX,
  },
  judge: {
    ahead: CircleMinus,
    never: undefined,
    current: CircleMinus,
    cleared: CircleCheck,
    waiting: CircleMinus,
    failed: CircleX,
  },
  human: {
    ahead: UserCheck,
    never: undefined,
    current: UserCheck,
    cleared: UserCheck,
    waiting: UserCheck,
    failed: UserCheck,
  },
};

/**
 * The glyph for a stage. Exported because the strip draws the same mark on the
 * chip that the card draws in its header, and two lookups would be two
 * answers.
 */
export function phaseGlyph(kind: PhaseStageKind, state: PhaseStageState): LucideIcon | undefined {
  return GLYPHS[kind][state];
}

/**
 * What each tier *is*, in one sentence, written once.
 *
 * Standing copy rather than values: true of every Job on every workflow, and a
 * screen that retyped them would be the second place the difference between a
 * Check and a Judge is stated. That difference is the whole reason the two
 * tiers are not one row of chips.
 */
export const SAID: Record<PhaseStageKind, string | undefined> = {
  phase: undefined,
  checks:
    "Commands this repository declares in its own Manifest. Fleet runs them and the Drone never " +
    "does — a Drone reporting its own tests is a claim, not a result.",
  judge:
    "A model reading the work against this step's acceptance criteria, the ones written when the " +
    "Job was dispatched. It answers per criterion, and it never sees the Drone's transcript, so it " +
    "cannot be argued at by the thing it is judging.",
  human:
    "The human gate, where the workflow asks for one. Everything mechanical has already cleared by " +
    "the time this tier is lit, so a step sitting here is stopped with nothing wrong.",
};

/** What each tier is worth knowing after the rows. Standing copy, same rule. */
export const CLOSES_WITH: Record<PhaseStageKind, string | undefined> = {
  phase: undefined,
  checks:
    "A command and an exit code. Nothing to interpret, and the same answer every time it is run.",
  judge: "It can only refuse. A Judge never turns a failed Check into a pass.",
  human: "Amber, not red. It is waiting on you, not broken.",
};

/**
 * A wire verdict, as the state a row stands in. `undefined` where the verdict
 * is one this does not know, which draws the stage's own state — an unmapped
 * verdict rendering neutrally is honest; rendering it green is not.
 */
export function verdictState(named: string | undefined): PhaseStageState | undefined {
  switch (named) {
    case "passed":
    case "met":
      return "cleared";
    case "failed":
    case "not_met":
    case "refused":
      return "failed";
    case "running":
      return "current";
    default:
      return undefined;
  }
}

/**
 * One row inside a card: a Check and its exit code, a criterion and its
 * verdict.
 */
export type PhaseCardRow = {
  /** The command, or the criterion. */
  label: ReactNode;
  /** What it came to — `exit 0 · 47s`, `running · 1m 04s`, `met`. */
  result?: ReactNode;
  /** Where the row stands, for its mark and its hue. */
  state?: PhaseStageState;
  /**
   * The verdict as the wire spells it — `passed`, `failed`, `met`, `not_met`,
   * `refused`. Used where `state` is absent, which is every caller that reads
   * the verdict straight off a `check_run` or a `judged` rather than
   * translating it first.
   *
   * **Not a second vocabulary.** The wire's spelling is what arrives, and a
   * caller rewriting it into `cleared` and `failed` before handing it over is
   * a translation that can go wrong somewhere nobody looks.
   */
  named?: string;
  /**
   * Whether `label` is machine-derived. A Check is a command, so it is mono; a
   * criterion is a sentence somebody wrote, so it is not.
   */
  mono?: boolean;
  /**
   * What the row cites — the evidence a refusal rests on. **The whole
   * persuasive content of a refusal**: a criterion id and `not_met` tell a
   * person nothing about their own Job.
   */
  cited?: ReactNode;
};

export type PhaseCardProps = {
  kind: PhaseStageKind;
  /** What the stage is called — `Checks`, `Judge`, `You`, `Submitted`. */
  name: ReactNode;
  state: PhaseStageState;
  /** Where it stands, in the caller's words — `1 of 2 · running`, `2 of 2 met`. */
  stands?: ReactNode;
  /**
   * What the tier is. Defaults to the standing sentence for its kind, which is
   * what a caller almost always wants; passing `null` draws none.
   */
  said?: ReactNode | null;
  /** The Checks and their exit codes, or the criteria and their verdicts. */
  rows?: PhaseCardRow[];
  /**
   * What the rows do not say, on the card's own well — `Approve, or send it
   * back with a reason. Both are recorded on the Job.`
   */
  note?: ReactNode;
  /** The closing line. Defaults to the standing one for its kind. */
  detail?: ReactNode | null;
  /**
   * Whether this is the card as it opens off the strip, rather than the same
   * content sitting flat on a panel.
   */
  floating?: boolean;
  /** Which edge the card is anchored to, and therefore where its arrow sits. */
  align?: "start" | "end";
};

/** The card's header mark is 16px; the row marks are 12px, at strokeWidth 2. */
const HEAD_GLYPH = 16;
const ROW_GLYPH = 12;
const STROKE = 2;

export function PhaseCard({
  kind,
  name,
  state,
  stands,
  said,
  rows = [],
  note,
  detail,
  floating,
  align = "start",
}: PhaseCardProps) {
  const Head = phaseGlyph(kind, state);
  const says = said === undefined ? SAID[kind] : said;
  const closes = detail === undefined ? CLOSES_WITH[kind] : detail;

  return (
    <div
      className="armada-phase-card"
      data-kind={kind}
      data-state={state}
      data-floating={floating || undefined}
      data-align={align}
    >
      <div className="armada-phase-card__head">
        {Head === undefined ? null : (
          <Head size={HEAD_GLYPH} strokeWidth={STROKE} className="armada-phase-card__mark" aria-hidden />
        )}
        <span className="armada-phase-card__name">{name}</span>
        {stands === undefined ? null : (
          <span className="armada-phase-card__stands">{stands}</span>
        )}
      </div>

      {says === null || says === undefined ? null : (
        <p className="armada-phase-card__said">{says}</p>
      )}

      {rows.length === 0 ? null : (
        <ul className="armada-phase-card__rows">
          {rows.map((row, at) => {
            const where = row.state ?? verdictState(row.named) ?? state;
            const Mark = phaseGlyph(kind, where);
            return (
              <li className="armada-phase-card__row" key={at}>
                <span className="armada-phase-card__row-line">
                  {Mark === undefined ? null : (
                    <Mark
                      size={ROW_GLYPH}
                      strokeWidth={STROKE}
                      className="armada-phase-card__row-mark"
                      data-state={where}
                      aria-hidden
                    />
                  )}
                  <span
                    className="armada-phase-card__row-label"
                    data-mono={row.mono ? "true" : undefined}
                  >
                    {row.label}
                  </span>
                  {row.result === undefined ? null : (
                    <span className="armada-phase-card__row-result">{row.result}</span>
                  )}
                </span>
                {row.cited === undefined ? null : (
                  <span className="armada-phase-card__cited">{row.cited}</span>
                )}
              </li>
            );
          })}
        </ul>
      )}

      {note === undefined ? null : <div className="armada-phase-card__note">{note}</div>}

      {closes === null || closes === undefined ? null : (
        <p className="armada-phase-card__detail">{closes}</p>
      )}
    </div>
  );
}
