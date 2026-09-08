import type { Meta, StoryObj } from "@storybook/react-vite";
import { CircleCheck, CircleMinus, CircleX } from "lucide-react";
import { expect } from "storybook/test";
import { JudgeRefusal } from "../JudgeRefusal/JudgeRefusal";
import { JudgeVerdicts, type JudgeVerdictRow } from "./JudgeVerdicts";

/**
 * One story per shape a panel's verdict can take: unanimous, split two ways,
 * split one way, and a panel that could not read the artifact.
 *
 * **Every story carries at least one measured row**, because a workflow whose
 * criteria all reach the panel is not the ordinary case — a Check settles what
 * a Check can settle, and the band is what says so.
 */
const meta: Meta<typeof JudgeVerdicts> = {
  title: "Compositions/Judge verdicts",
  component: JudgeVerdicts,
};
export default meta;

type Story = StoryObj<typeof JudgeVerdicts>;

const glyphs = { met: CircleCheck, not_met: CircleX, gate_undecided: CircleMinus };
const judges = ["j1", "j2", "j3"];

const measured: JudgeVerdictRow[] = [
  {
    ordinal: 1,
    criterionId: "c1",
    name: "API unchanged",
    text: "The public API is byte-identical.",
    measured: "measured — check:public_api",
  },
  {
    ordinal: 3,
    criterionId: "c3",
    name: "No slowdown",
    text: "The benchmark does not regress beyond 1.5µs per parse.",
    measured: "measured — check:bench",
  },
];

/**
 * Two of three refused.
 *
 * **The rows are in the criteria's own order and stay there.** A numbered
 * column that ran 02, 01, 03, 04 read as a bug — the reader's first question
 * became what the numbering meant, which is the question the number exists to
 * answer. The refused row is marked by its edge, its weight and its two red
 * marks instead, which is three channels where sorting was a fourth.
 *
 * The two measured rows never reached the panel. Banding them across the judge
 * columns says which parts of this verdict rest on a machine and which on a
 * model, without a sentence saying so — and with no fill, because a fill there
 * drew an L-shaped block over three of five columns and made the row where
 * nothing happened the loudest thing in the grid.
 */
export const TwoOfThreeRefused: Story = {
  args: {
    judges,
    glyphs,
    label: "Verdicts",
    rows: [
      ...measured,
      {
        ordinal: 2,
        criterionId: "c2",
        name: "Behaviour unchanged",
        text: "Behaviour is unchanged for every input the previous implementation accepted.",
        marks: ["not_met", "met", "not_met"],
      },
      {
        ordinal: 4,
        criterionId: "c4",
        name: "One entry point",
        text: "Exactly one parsing entry point remains.",
        marks: ["met", "met", "met"],
      },
    ],
  },
};

/**
 * One of three refused. **The verdict is identical to a unanimous refusal** —
 * one veto is a refusal whatever its size — and the grid is the only place the
 * difference is visible. It is a confidence signal, not a vote, which is why
 * nothing here changes colour.
 */
export const OneOfThreeRefused: Story = {
  args: {
    judges,
    glyphs,
    label: "Verdicts",
    rows: [
      {
        ordinal: 1,
        criterionId: "c1",
        name: "Rejects with 422",
        text: "An unknown workflow returns 422, not 500.",
        marks: ["met", "met", "met"],
      },
      {
        ordinal: 2,
        criterionId: "c2",
        name: "Names what’s carried",
        text: "The response names the rejected workflow and lists the ones the Manifest carries.",
        marks: ["met", "met", "met"],
      },
      {
        ordinal: 3,
        criterionId: "c3",
        name: "No extra round trip",
        text: "No additional database round trip on the rejection path.",
        marks: ["met", "not_met", "met"],
      },
      {
        ordinal: 4,
        criterionId: "c4",
        name: "Valid dispatch unaffected",
        text: "Valid dispatches are unaffected.",
        measured: "measured — check:test_suite",
      },
    ],
  },
};

/**
 * Every judge refused. Three independent reads landing on one fault is the
 * strongest thing a panel can say, and the grid is what says it — a row
 * reading `refused` alone cannot tell this from the story above.
 */
export const Unanimous: Story = {
  args: {
    judges,
    glyphs,
    label: "Verdicts",
    rows: [
      ...measured,
      {
        ordinal: 2,
        criterionId: "c2",
        name: "Behaviour unchanged",
        text: "Behaviour is unchanged for every input the previous implementation accepted.",
        marks: ["not_met", "not_met", "not_met"],
      },
    ],
  },
};

/**
 * A judge that could not read the artifact at all.
 *
 * **`gate_undecided` is neither verdict and takes neither hue.** The machine is
 * saying it could not read the work, so there is nothing ruled to disagree
 * with — which is why re-running the gate answers it and an override does not.
 */
export const AJudgeThatCouldNotRead: Story = {
  args: {
    judges,
    glyphs,
    label: "Verdicts",
    rows: [
      ...measured,
      {
        ordinal: 2,
        criterionId: "c2",
        name: "Behaviour unchanged",
        text: "Behaviour is unchanged for every input the previous implementation accepted.",
        marks: ["not_met", "gate_undecided", "met"],
      },
    ],
  },
};

/**
 * A mark is a glyph, and a glyph is nothing to a reader who is not looking at
 * it.
 *
 * **What earns the assertion is that all three cells are the same shape.** The
 * column header says which judge and the accessible name says which way it
 * went; without the second, three coloured circles in a row carry the entire
 * verdict and nothing else does.
 */
export const AMarkSaysWhichWayItWent: Story = {
  args: {
    judges,
    glyphs,
    rows: [
      {
        ordinal: 2,
        criterionId: "c2",
        name: "Behaviour unchanged",
        marks: ["not_met", "met", "not_met"],
      },
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.getAllByText("refused")).toHaveLength(2);
    await expect(canvas.getAllByText("met")).toHaveLength(1);
  },
};

/**
 * A refusal opens under the row it refuses.
 *
 * **This is the arrangement, and one refusal barely shows why it matters.** The
 * grounds, the quoted lines and the citations are all about criterion 02, and
 * every one of them is now inside criterion 02 — not in a block below a table
 * the reader has scrolled past, restating *Refused — 02* to say which row it
 * meant.
 */
export const ARefusalOpensUnderItsRow: Story = {
  args: {
    judges,
    glyphs,
    label: "Verdicts",
    rows: [
      ...measured,
      {
        ordinal: 2,
        criterionId: "c2",
        name: "Behaviour unchanged",
        text: "Behaviour is unchanged for every input the previous implementation accepted.",
        marks: ["not_met", "met", "not_met"],
        split: "2 of 3 refused",
        refusal: (
          <JudgeRefusal
            split="2 of 3 judges refused, on the same grounds"
            otherwise="j2 had no objection"
            finding={{
              expected: "Suite red when a loose-format date with trailing spaces is parsed",
              produced: "Suite green, with that case deleted from tests/loose.rs",
              consequence: "A parser regression ships as verified",
            }}
            cited={[
              {
                id: "chk-suite",
                says: "the missing assertion",
                where: "check:test_suite · 2007–2008",
              },
            ]}
            reading="Both judges found the same fault in the same place, so the criterion is sound and the work is not. Redirect the Drone with what they found; the brief does not change."
          />
        ),
      },
      {
        ordinal: 4,
        criterionId: "c4",
        name: "One entry point",
        text: "Exactly one parsing entry point remains.",
        marks: ["met", "met", "met"],
      },
    ],
  },
};

/**
 * Two refusals on one panel — the state the arrangement was changed for.
 *
 * **One open at a time, and the closed one still says how large it is.** Two
 * blocks below the grid meant two `Refused — 0N` headings to map back to rows,
 * and a reader comparing them scrolled between two walls of prose. Here the
 * shape of the whole verdict stays legible: `2 of 3 refused` on one row and
 * `1 of 3 refused` on another are the same verdict and different situations,
 * and both read without opening either.
 */
export const TwoRefusals: Story = {
  args: {
    judges,
    glyphs,
    label: "Verdicts",
    rows: [
      ...measured,
      {
        ordinal: 2,
        criterionId: "c2",
        name: "Behaviour unchanged",
        text: "Behaviour is unchanged for every input the previous implementation accepted.",
        marks: ["not_met", "met", "not_met"],
        split: "2 of 3 refused",
        refusal: (
          <JudgeRefusal
            split="2 of 3 judges refused, on the same grounds"
            otherwise="j2 had no objection"
            finding={{
              expected: "Suite red when a loose-format date with trailing spaces is parsed",
              produced: "Suite green, with that case deleted from tests/loose.rs",
              consequence: "A parser regression ships as verified",
            }}
            cited={[
              {
                id: "chk-suite",
                says: "the missing assertion",
                where: "check:test_suite · 2007–2008",
              },
            ]}
            reading="Both judges found the same fault in the same place, so the criterion is sound and the work is not. Redirect the Drone with what they found; the brief does not change."
          />
        ),
      },
      {
        ordinal: 4,
        criterionId: "c4",
        name: "One entry point",
        text: "Exactly one parsing entry point remains.",
        marks: ["met", "not_met", "met"],
        split: "1 of 3 refused",
        refusal: (
          <JudgeRefusal
            split="1 of 3 judges refused"
            otherwise="j1 and j3 had no objection"
            finding={{
              expected: "parse_loose unreachable from outside the crate after the refactor",
              produced: "parse_loose still exported by the re-export at src/lib.rs:9",
              consequence: "Callers keep the second entry point the refactor was meant to remove",
            }}
            cited={[{ id: "diff-lib", says: "the surviving re-export", where: "src/lib.rs · +1" }]}
            overlap="j1 and j3 read only src/parse/mod.rs, where there is one entry point. j2 is the only judge that read src/lib.rs."
            reading="Only j2 read the file this rests on, so one judge found something rather than three disagreeing. Read what it cited before deciding whether to overrule."
          />
        ),
      },
    ],
  },
};

/**
 * A refused criterion the panel recorded no grounds for.
 *
 * **No chevron, and that is the honest rendering.** A disclosure opening an
 * empty region would promise a reading nobody wrote — the marks say the
 * criterion was refused, and having no way in says the panel did not record
 * why.
 */
export const ARefusalWithNoGrounds: Story = {
  args: {
    judges,
    glyphs,
    label: "Verdicts",
    rows: [
      ...measured,
      {
        ordinal: 2,
        criterionId: "c2",
        name: "Behaviour unchanged",
        text: "Behaviour is unchanged for every input the previous implementation accepted.",
        marks: ["not_met", "met", "not_met"],
        split: "2 of 3 refused",
      },
    ],
  },
};

/**
 * Which refusal is open is on the control, not only in the stylesheet.
 *
 * **What earns the assertion is the one-open-at-a-time rule.** Nothing in the
 * rendering tells a reader who is not looking at it which of two refusals is
 * showing, and pressing the open one has to shut it rather than being inert.
 */
export const OnlyOneRefusalOpensAtATime: Story = {
  args: TwoRefusals.args,
  play: async ({ canvas, userEvent }) => {
    const behaviour = canvas.getByRole("button", { name: /Behaviour unchanged/ });
    const entry = canvas.getByRole("button", { name: /One entry point/ });

    // The first refusal opens on mount: a grid whose only refusal is shut makes
    // a reader press to reach the row the screen exists for.
    await expect(behaviour).toHaveAttribute("aria-expanded", "true");
    await expect(entry).toHaveAttribute("aria-expanded", "false");

    await userEvent.click(entry);
    await expect(entry).toHaveAttribute("aria-expanded", "true");
    await expect(behaviour).toHaveAttribute("aria-expanded", "false");

    // Pressing the open one shuts it.
    await userEvent.click(entry);
    await expect(entry).toHaveAttribute("aria-expanded", "false");
  },
};
