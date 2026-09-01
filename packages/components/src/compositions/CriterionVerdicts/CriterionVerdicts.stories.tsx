import type { Meta, StoryObj } from "@storybook/react-vite";
import { CircleCheck, CircleX, ShieldCheck } from "lucide-react";
import { WorkflowRail } from "../WorkflowRail/WorkflowRail";
import { CriterionVerdicts, type CriterionVerdict } from "./CriterionVerdicts";

/**
 * One story per state the contract names: a refusal with its citation, a
 * criterion the Judge did not object to, a step with both — which is the state
 * that proves refusals sort first while the numbers stay frozen — and a
 * refusal whose criterion text never reached the screen.
 *
 * The glyphs are the `circle-*` family the Judge owns and the verbs are
 * `criterion_verdict_judge`'s own: "no objection", never "accepted", because
 * the Judge declines to refuse and never grants.
 */
const meta: Meta<typeof CriterionVerdicts> = {
  title: "Compositions/Criterion verdicts",
  component: CriterionVerdicts,
};
export default meta;

type Story = StoryObj<typeof CriterionVerdicts>;

const MET = CircleCheck;
const NOT_MET = CircleX;

const refused: CriterionVerdict = {
  ordinal: 2,
  criterionId: "c2",
  text: "A failed refresh signs the session out.",
  named: "not_met",
  verdict: "refused",
  icon: NOT_MET,
  expected: "A 401 from the refresh endpoint clears the session and returns the caller to sign-in.",
  produced:
    "The refresh error is swallowed in `session.ts:212` and the stale token is retried on the next request.",
  consequence:
    "A user whose refresh token has been revoked keeps a working-looking session until the next full reload, so a revoked device is not signed out.",
};

const met: CriterionVerdict = {
  ordinal: 1,
  criterionId: "c1",
  text: "Expired tokens refresh once rather than per request.",
  named: "met",
  verdict: "no objection",
  icon: MET,
};

/**
 * The refusal, whole. Three labelled lines rather than a paragraph: the fields
 * arrive named from the Judge record, and `consequence` carries the weight
 * because it is the line a person triages on.
 *
 * **This is not a failed Check and is drawn so it cannot be read as one.** No
 * `shield-*` glyph, no exit code, no command — a Check says the work is broken
 * and this says the work runs and is not what was asked for.
 */
export const ARefusal: Story = {
  args: { rows: [refused] },
};

/** A criterion the Judge did not object to. One line, and nothing is owed. */
export const NoObjection: Story = {
  args: { rows: [met] },
};

/**
 * Both, on one step. The refusal sorts to the top and keeps its number, so a
 * citation to "criterion 2" still resolves against the frozen order it was
 * written against.
 */
export const RefusalsSortFirst: Story = {
  args: {
    label: "What the judge answered",
    rows: [
      met,
      refused,
      {
        ordinal: 3,
        criterionId: "c3",
        text: "The fix carries a regression test.",
        named: "met",
        verdict: "no objection",
        icon: MET,
      },
    ],
  },
};

/**
 * A refusal citing a criterion the Job does not carry — `acceptance_criteria`
 * has no such id. The id stands in, in mono, and the row loses its number
 * rather than taking its place on screen as one: a guessed position would
 * break the reference a retry is written against. The citation still reads.
 */
export const TheCriterionIsNotOnScreen: Story = {
  args: {
    rows: [
      {
        criterionId: "c4",
        named: "not_met",
        verdict: "refused",
        icon: NOT_MET,
        expected: "The migration is reversible.",
        produced: "`down()` is empty.",
        consequence: "A bad deploy cannot be rolled back without restoring from a snapshot.",
      },
    ],
  },
};

/**
 * A verdict the registry has no glyph or verb for. Nothing is invented in its
 * place — the number, the criterion and the hue are what there is, and the
 * missing pieces are visible rather than papered over.
 */
export const TheRegistryHasNoWordForIt: Story = {
  args: {
    rows: [
      { ordinal: 1, criterionId: "c1", text: "The fix addresses the cause.", named: "unknown" },
    ],
  },
};

/**
 * The brief, on a refusal and on a pass alike.
 *
 * **A verdict that cannot be re-read against its input is one nobody can argue
 * with.** The path is the whole of what Bridge shows — a brief carries the
 * request, the deliverable and the entire branch diff, and Bridge does not read
 * the filesystem — so it is mono, subtle, and copies on click, exactly as a
 * Check's output path does one row up.
 *
 * **On the met row too, and that is the point.** A Judge that refuses wrongly
 * is argued with the same day; a Judge that *passes* something it should have
 * refused is the quiet failure, and that one is only visible against what it
 * was shown. It sits at the end of the head line, so a met verdict is still one
 * line.
 */
export const WhereTheBriefWasKept: Story = {
  args: {
    label: "What the judge answered",
    rows: [
      { ...met, briefPath: ".armada/briefs/01JOB/implement.1.c1.txt" },
      { ...refused, briefPath: ".armada/briefs/01JOB/implement.1.c2.txt" },
    ],
  },
};

/**
 * Where these actually draw: beneath the step the Judge judged, in the rail on
 * job detail.
 *
 * **Told apart from the gate row above it three ways, none of them a status
 * label** — the `circle-*` glyph family, which `icons.toml` reserves to the
 * Judge and refuses to a Check; the criterion's own words in sans where a gate
 * row carries a command in mono; and three named fields where a gate row
 * carries an exit code.
 *
 * **The refusal sits under the step and never on it.** Verdict hue is per
 * criterion and never sums onto the step or the Job, so the step keeps its own
 * activity while a red cross sits beneath it.
 */
export const BeneathTheStepItJudged: StoryObj<typeof WorkflowRail> = {
  render: () => (
    <WorkflowRail
      steps={[
        { id: "plan", label: "Plan the change", activity: "advanced", status: "advanced" },
        {
          id: "implement",
          label: "Implement",
          activity: "awaiting_human",
          status: "waiting on you",
          current: true,
          gates: [
            {
              command: "build · cargo build --workspace",
              result: "exit 0",
              icon: ShieldCheck,
              iconLabel: "Passed",
            },
          ],
          verdicts: [met, refused],
        },
        { id: "verify", label: "Run tests", activity: "not_started", status: "not started" },
      ]}
    />
  ),
};
