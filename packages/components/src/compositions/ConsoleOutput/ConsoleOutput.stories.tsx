import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";
import { Button } from "../../primitives/Button/Button";
import { ConsoleOutput, type ConsoleRow } from "./ConsoleOutput";

/**
 * One story per state the viewer has to hold: the reading a selector lands on,
 * the same reading expanded with its tools, a check still writing, and a check
 * that wrote nothing.
 *
 * **The transcript in the first three is one file**, so the difference between
 * the stories is what the viewer does with it rather than what it was given.
 */
const meta: Meta<typeof ConsoleOutput> = {
  title: "Compositions/Console output",
  component: ConsoleOutput,
};
export default meta;

type Story = StoryObj<typeof ConsoleOutput>;

/**
 * The region a selector lands on: the four lines around the difference, the
 * compile folded away, and the 309 tests after it folded away too.
 */
const around: ConsoleRow[] = [
  { row: "fold", at: 1940, says: "compiling chrono-lite v0.4.2 (18.4s) — 62 lines" },
  { row: "fold", at: 2003, says: "running 315 tests", open: true },
  { row: "line", at: 2004, text: "test parses_rfc3339_offset ... ok" },
  { row: "line", at: 2005, text: "test parses_rfc2822_obsolete_zone ... ok" },
  { row: "line", at: 2006, text: "test parses_loose_no_separator ... ok" },
  {
    row: "line",
    at: 2007,
    against: "marker",
    text: "— 1 test present at HEAD~1 is absent here —",
  },
  {
    row: "line",
    at: 2008,
    against: "absent",
    text: "test parses_loose_trailing_whitespace ... ok",
    note: "ran and passed at HEAD~1 · never ran here",
  },
  { row: "line", at: 2009, text: "test rejects_empty_string ... ok" },
  { row: "line", at: 2010, text: "test rejects_year_zero ... ok" },
  { row: "fold", at: 2011, says: "309 further tests, all ok — 309 lines" },
  { row: "line", at: 2321, text: "test result: ok. 315 passed; 0 failed; finished in 2.41s" },
  { row: "line", at: 2322, text: "at HEAD~1: 316 passed; 0 failed; finished in 2.38s" },
];

/**
 * The inline reading — the size that composes the page.
 *
 * **The last two lines are the design.** A transcript ending `315 passed` is a
 * fact with no reference point; the same transcript with the parent commit's
 * count beneath it is an argument. The check emitted both, so the viewer only
 * renders what is already in the file.
 *
 * **No tools at this size.** A tool bar here would make a quoted region look
 * like a place to work.
 */
export const TheReadingASelectorLandsOn: Story = {
  args: {
    rows: around,
    region: {
      says: "lines 1,940–2,322 of 2,180",
      path: ".armada/jobs/733/checks/test_suite.log",
      size: "4.1MB",
      why: "jumped to first difference",
    },
  },
};

/**
 * Expanded. The tools appear, the search says how many of its matches are
 * inside folds, and the folds holding them are open.
 *
 * **`3 in 2 folds` is the line that matters.** A search that reported three
 * matches and showed one is the failure this component exists not to have —
 * the count names where the rest are, and the folds carrying them opened.
 */
export const ExpandedWithItsTools: Story = {
  args: {
    rows: [
      { row: "fold", at: 1996, says: "running 315 tests", open: true },
      { row: "line", at: 1997, text: "test parses_rfc3339_offset ... ok" },
      { row: "line", at: 1998, text: "test parses_rfc3339_z ... ok" },
      { row: "line", at: 1999, text: "test parses_loose_no_separator ... ok" },
      {
        row: "line",
        at: 2007,
        against: "marker",
        text: "— 1 test present at HEAD~1 is absent here —",
      },
      {
        row: "line",
        at: 2008,
        against: "absent",
        landed: true,
        text: (
          <>
            {"test parses_loose_"}
            <mark>trailing</mark>
            {"_whitespace ... ok"}
          </>
        ),
        note: "ran and passed at HEAD~1 · deleted in tests/loose.rs",
      },
      { row: "line", at: 2009, text: "test rejects_empty_string ... ok" },
      {
        row: "fold",
        at: 2012,
        open: true,
        says: (
          <>
            {"tests/loose.rs — 2 matches for “"}
            <mark>trailing</mark>
            {"”"}
          </>
        ),
      },
      { row: "line", at: 2013, text: "test loose_ignores_leading_space ... ok" },
      {
        row: "line",
        at: 2014,
        text: (
          <>
            {"// removed: "}
            <mark>trailing</mark>
            {" whitespace handled by caller now"}
          </>
        ),
      },
      { row: "fold", at: 2015, says: "306 further tests, all ok — 306 lines" },
      { row: "line", at: 2321, text: "test result: ok. 315 passed; 0 failed; finished in 2.41s" },
      { row: "line", at: 2322, text: "at HEAD~1: 316 passed; 0 failed; finished in 2.38s" },
    ],
    region: {
      says: "lines 1,996–2,322 of 2,180",
      why: "folds containing matches are opened automatically",
    },
    tools: {
      find: { term: "trailing", found: "3 in 2 folds" },
      differences: { says: "1 of 1 difference" },
      acts: (
        <>
          <Button size="sm" variant="secondary" ground="sunken">
            Wrap
          </Button>
          <Button size="sm" variant="secondary" ground="sunken">
            Raw
          </Button>
          <Button size="sm" variant="secondary" ground="sunken">
            Reveal in Finder
          </Button>
        </>
      ),
    },
  },
};

/**
 * A check still writing. **The only things that differ from a finished check
 * are that the last line moves and there is no verdict yet** — so it is the
 * same viewer, not a second one.
 *
 * The region line sits under the reading here rather than over it, beside the
 * moving edge, which is the part being watched.
 */
export const StillWriting: Story = {
  args: {
    following: true,
    rows: [
      { row: "fold", at: 1, says: "compiling core-api v2.9.0 (41.8s) — 118 lines" },
      { row: "fold", at: 119, says: "running 1,204 tests", open: true },
      { row: "line", at: 1122, text: "test dispatch::rejects_unknown_workflow ... ok" },
      { row: "line", at: 1123, text: "test dispatch::lists_carried_workflows ... ok" },
      { row: "line", at: 1124, text: "test dispatch::accepts_known_workflow ... ok" },
      { row: "line", at: 1125, text: "test resolver::unknown_is_edge_rejected ... running" },
    ],
    region: {
      says: "1,124 of 1,204 · 6.1s elapsed",
      why: "follow pauses when you scroll up",
    },
  },
};

/**
 * A check that wrote nothing. **A named absence rather than an empty frame** —
 * a viewer with no rows in it and no sentence reads as a viewer that is still
 * loading.
 */
export const NothingWritten: Story = {
  args: {
    rows: [],
    emptyNote: "This check wrote no output before it exited.",
    region: { says: "0 lines", path: ".armada/jobs/733/checks/query_budget.log" },
  },
};

/**
 * A fold states what it is hiding and says whether it is open.
 *
 * **What earns the assertion is that a closed fold and an open one are the
 * same row.** The caret is one character in a mono column, so nothing in the
 * rendering carries the state to a reader who is not looking at it —
 * `aria-expanded` is the property that should hold it and does.
 */
export const AFoldSaysWhetherItIsOpen: Story = {
  args: {
    rows: [
      { row: "fold", at: 1940, says: "compiling chrono-lite v0.4.2 (18.4s) — 62 lines" },
      { row: "fold", at: 2003, says: "running 315 tests", open: true },
      { row: "line", at: 2004, text: "test parses_rfc3339_offset ... ok" },
    ],
  },
  play: async ({ canvas }) => {
    const closed = canvas.getByRole("button", {
      name: /compiling chrono-lite v0\.4\.2 \(18\.4s\) — 62 lines/,
    });
    await expect(closed).toHaveAttribute("aria-expanded", "false");

    const open = canvas.getByRole("button", { name: /running 315 tests/ });
    await expect(open).toHaveAttribute("aria-expanded", "true");
  },
};
