import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";
import { JudgeInputs, type JudgeInputRow, type JudgeInputsFor } from "./JudgeInputs";

/**
 * The view exists to answer one question — was this a panel, or three judges
 * shown different things — and then to let a reader go and check.
 *
 * **The second and third stories are the reason it has a segmented control.**
 * Being told *j2 was shown something else* without being able to see what j2
 * was shown is the least useful place to stop.
 */
const meta: Meta<typeof JudgeInputs> = {
  title: "Compositions/Judge inputs",
  component: JudgeInputs,
};
export default meta;

type Story = StoryObj<typeof JudgeInputs>;

const SHARED: JudgeInputRow[] = [
  { name: "scope digest", value: "sha256:9f31c2…a70b" },
  { name: "context_paths", value: "src/parse/mod.rs\nsrc/parse/loose.rs\ntests/loose.rs" },
  { name: "work product", value: "diff, delivered — 5 files, 412 lines" },
  { name: "yardstick", value: "acceptance_criteria[] frozen at dispatch" },
  { name: "facts", value: "3 check results, pre-loaded" },
  { name: "size", value: "38.2k of 40k max_context_size" },
];

/** Every judge handed the same object, so every per-judge view is the same. */
const AGREED: JudgeInputsFor[] = ["j1", "j2", "j3"].map((judge) => ({
  id: judge,
  judge,
  rows: SHARED,
}));

/**
 * One object, three judges. **The digest is the evidence for independence**,
 * and it is the first thing to doubt when a unanimous verdict looks too easy.
 *
 * The per-judge views are identical here, and that is not redundancy — it is
 * the claim, checkable.
 */
export const OneObjectEveryJudgeSaw: Story = {
  args: {
    label: "Inputs",
    rows: SHARED,
    each: AGREED,
    identical: "All 3 judges received this identical object",
  },
};

/**
 * Judges shown different objects — and now you can open j2 and see what it
 * actually got.
 *
 * **The verdict above this is not a panel's verdict.** Nothing else on the
 * screen can say that: the grid renders three columns of marks either way, and
 * unanimity between judges who read different things means nothing at all.
 */
export const JudgesShownDifferentObjects: Story = {
  args: {
    label: "Inputs",
    rows: [
      {
        name: "scope digest",
        value: "j1, j3 — sha256:9f31c2…a70b\nj2 — sha256:41ba07…cc19",
        differs: true,
      },
      { name: "context_paths", value: "j2 was not given tests/loose.rs", differs: true },
      { name: "work product", value: "diff, delivered — 5 files, 412 lines" },
      { name: "yardstick", value: "acceptance_criteria[] frozen at dispatch" },
      { name: "size", value: "38.2k of 40k max_context_size · j2 34.9k", differs: true },
    ],
    differed:
      "j2 was shown a different object. Three judges that read different things are not a panel, and their agreement is not independence.",
    each: [
      { id: "j1", judge: "j1", rows: SHARED },
      {
        id: "j2",
        judge: "j2",
        rows: [
          { name: "scope digest", value: "sha256:41ba07…cc19", differs: true },
          { name: "context_paths", value: "src/parse/mod.rs\nsrc/parse/loose.rs", differs: true },
          { name: "work product", value: "diff, delivered — 5 files, 412 lines" },
          { name: "yardstick", value: "acceptance_criteria[] frozen at dispatch" },
          { name: "facts", value: "3 check results, pre-loaded" },
          { name: "size", value: "34.9k of 40k max_context_size", differs: true },
        ],
      },
      { id: "j3", judge: "j3", rows: SHARED },
    ],
  },
};

/**
 * The same panel, opened on j2 — the object that differs.
 *
 * **The differing fields stay marked here.** Read on its own, nothing about a
 * digest says it is the odd one out, so the mark is what carries the
 * comparison into the view that has lost it.
 */
export const OpenedOnTheJudgeThatDiffered: Story = {
  args: { ...JudgesShownDifferentObjects.args, view: "j2" } as Story["args"],
};

/**
 * A panel where only the comparison was recorded. **No segmented control**,
 * because a segment per judge that opened the same summary would promise a
 * reading nobody kept.
 */
export const OnlyTheComparisonWasKept: Story = {
  args: {
    label: "Inputs",
    rows: SHARED,
    identical: "All 3 judges received this identical object",
  },
};

/**
 * The segmented control opens a judge's own object, and the assurance belongs
 * to the comparison alone.
 *
 * **What earns the assertion is the second half.** The assurance is a claim
 * about the *panel*; leaving it above one member's object would attach it to a
 * reading it is not about, and nothing in the rendering would say so.
 */
export const AJudgesOwnObject: Story = {
  args: JudgesShownDifferentObjects.args,
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getByText(/j2 was shown a different object/)).toBeVisible();

    await userEvent.click(canvas.getByRole("tab", { name: "j2" }));

    // j2's own digest, not the comparison's two-line summary.
    await expect(canvas.getByText("sha256:41ba07…cc19")).toBeVisible();
    await expect(canvas.queryByText(/j2 was shown a different object/)).toBeNull();
  },
};
