import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { DroneBrief } from "./DroneBrief";

/**
 * The brief off a real `plan` step, block for block as
 * `crates/fleet/src/briefing.rs` composes it.
 *
 * **A fixture, not a sample.** The headings, the two-space indent on a list
 * item and the five-space indent under the current part are all its own, and a
 * tidied copy would be a drawing of a brief nobody is ever sent.
 */
const BRIEF = [
  "JOB BRIEF",
  "",
  "Coalesce concurrent token refreshes",
  "",
  "Two requests arriving inside the refresh window each start their own refresh,",
  "and the second overwrites the first's token.",
  "",
  "This is done when:",
  "  - two refreshes in flight make one network call",
  "  - the second caller waits on the first rather than starting its own",
  "",
  "WHERE YOU ARE",
  "",
  "This task runs in 4 parts. You are on part 1.",
  "",
  "  1. Plan the change — you are here",
  "     STOP. Submit when this part is done, then wait.",
  "  2. Implement — not yours — do not do it",
  "  3. Verify — not yours — do not do it",
  "  4. Hand off — not yours — do not do it",
  "",
  "The parts after this one happen after you submit, and doing them yourself does",
  "not move this task forward. Leave the branch in a state they can start from.",
  "",
  "STEP: Plan the change",
  "",
  "What you claim should be what the work now does, not that you finished. An",
  "adjacent problem you notice and leave alone goes under Not claimed.",
].join("\n");

/**
 * The delivery block, with a target long enough to reach the panel's edge.
 * `artifact_exists` takes whatever path a workflow declares, so the length is
 * the workflow author's and not this component's to assume small.
 */
const DELIVERS = [
  "WHAT THIS PART DELIVERS",
  "",
  "Write this part's finding to a file, at this exact path in your worktree:",
  "",
  "  .armada/artifacts/coalesce-concurrent-token-refreshes-root-cause-and-plan.md",
  "",
  "This is the work product, not a note to yourself. This exact path is the one",
  "that is read: an empty file or no file stops this part.",
].join("\n");

/**
 * What Armada told the Drone, in the blocks Fleet wrote it in.
 *
 * **Drawn on the chapter body it lives on, not on the canvas.** The brief is
 * chapter one of a step's story, which is a `--bg-sunken` well inside a
 * `--bg-raised` panel at `--text-xs`. Judged on the canvas it would be judged
 * at the wrong size against the wrong ground, and the question this component
 * exists to answer — whether a two-space indent still reads as a list — is a
 * question about that size and that width.
 */
const meta: Meta<typeof DroneBrief> = {
  title: "Compositions/Drone brief",
  component: DroneBrief,
  decorators: [
    (Story) => (
      <div
        style={{
          width: "calc(var(--space-12) * 12)",
          padding: "var(--space-4)",
          borderRadius: "var(--radius-md)",
          border: "var(--border-width) solid var(--border-default)",
          background: "var(--bg-raised)",
        }}
      >
        <div
          style={{
            padding: "calc(var(--space-3) + var(--space-1) / 2)",
            borderRadius: "var(--radius-md)",
            border: "var(--border-width) solid var(--border-subtle)",
            background: "var(--bg-sunken)",
            fontSize: "var(--text-xs)",
            lineHeight: "var(--leading-xs)",
          }}
        >
          <Story />
        </div>
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof DroneBrief>;

/**
 * The brief as Fleet writes it on a `plan` step of the `bug` workflow. **The
 * state this component exists for**, and the state #306 was reported against:
 * three blocks, a parts list, and a `STOP.` under the part the Drone is on.
 *
 * Drawn as one paragraph, every heading ran inline with the prose around it and
 * the stop boundary landed mid-sentence. Nothing about the payload changed to
 * fix it — the newlines were on the wire and in the DOM the whole time.
 *
 * **The rail is drawn at a wider indent than Fleet wrote it at.** Two spaces is
 * seven pixels at this size and read as nudged prose rather than as a list, so
 * `widenIndent` doubles every one. The cost is that this rail and the
 * transcript Fleet sent are no longer the same string, and `DroneBrief.tsx`
 * says so where it happens.
 */
export const TheBriefAsFleetWroteIt: Story = {
  args: { lines: BRIEF.split("\n") },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    // Each heading is the whole of its own element, which is what "on its own
    // line" is when a test has to read it. Drawn as one paragraph no element
    // has this text and every one of these throws.
    const brief = canvas.getByText("JOB BRIEF");
    const where = canvas.getByText("WHERE YOU ARE");
    const step = canvas.getByText("STEP: Plan the change");

    // And they are three lines rather than three readings of one, taken off
    // what the browser drew rather than off the markup.
    await expect(brief.getBoundingClientRect().top).toBeLessThan(
      where.getBoundingClientRect().top,
    );
    await expect(where.getBoundingClientRect().top).toBeLessThan(step.getBoundingClientRect().top);

    // The parts rail, reached through a heading rather than by naming a class:
    // a test that names markup fails on every refactor and says nothing about
    // what a person saw.
    const blocks = [...(where.parentElement?.children ?? [])] as HTMLElement[];
    const rail = blocks.find((block) => block.textContent?.includes("STOP."));
    if (rail === undefined) throw new Error("the parts rail is not a block of its own");

    // The boundary is still a line of its own, under the part it belongs to,
    // and still indented past it. Ten spaces and not the five Fleet wrote:
    // `widenIndent` doubles every one, which is the divergence it documents.
    await expect(rail.textContent).toContain(
      "\n          STOP. Submit when this part is done, then wait.",
    );

    // `white-space` is the declaration #306 was: the newlines above are in the
    // DOM either way, and only this decides whether a reader sees them. Read
    // off the rendering rather than off the class attribute.
    await expect(getComputedStyle(rail).whiteSpace).toBe("pre-wrap");

    // Five lines drawn as more than two. A lower bound, so a rail read at a
    // narrower width still passes.
    const line = parseFloat(getComputedStyle(rail).lineHeight);
    await expect(rail.getBoundingClientRect().height).toBeGreaterThan(line * 2);
  },
};

/**
 * A later turn. **Armada opens a step with the brief above and speaks again
 * several times after it** — the gate's outcome handed back, a person's
 * redirect carried in — and those are one block of prose with no boundary in
 * them.
 *
 * There is no heading here and none is invented. A component that drew the
 * first line of every payload as a heading would draw this sentence's first
 * clause as one.
 */
export const OneBlockAndNoBoundary: Story = {
  args: {
    lines: [
      "The gate refused this step. Check tests exited 1: 4 assertions failed in " +
        "packages/settings/src/selectors.test.ts. Fix what it found and submit again.",
    ],
  },
};

/**
 * The delivery block, whose whole content is a path.
 *
 * **The decision `white-space: pre-wrap` forces.** It restores the newlines and
 * the indent, and it hands the line length to whatever `briefing.rs` wrote — so
 * a path longer than the panel would decide the panel. `overflow-wrap:
 * anywhere` is the answer: it breaks mid-token only where the whole run does
 * not fit, which is worse than a break at a slash and better than a path
 * nobody can read.
 */
export const ADeliveryPathLongerThanThePanel: Story = {
  args: { lines: DELIVERS.split("\n") },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const blocks = [
      ...(canvas.getByText("WHAT THIS PART DELIVERS").parentElement?.children ?? []),
    ] as HTMLElement[];
    const path = blocks.find((block) => block.textContent?.includes(".armada/artifacts/"));
    if (path === undefined) throw new Error("the delivery path is not a block of its own");
    // Nothing is behind the edge. `scrollWidth` past `clientWidth` is content
    // the panel is holding and not drawing, which is the failure mode.
    await expect(path.scrollWidth).toBeLessThanOrEqual(path.clientWidth + 1);
  },
};
