import type { Decorator, Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";

import { WorkflowDiagram, type WorkflowDiagramStep } from "./WorkflowDiagram";

/**
 * One story per shape the design session named: a straight line ending in a
 * stop for a person, and a loop back to an earlier step.
 */
const meta: Meta<typeof WorkflowDiagram> = {
  title: "Compositions/Workflow diagram",
  component: WorkflowDiagram,
};
export default meta;

type Story = StoryObj<typeof WorkflowDiagram>;

/** `bug.json`, as M1 ships it: a straight line, two Judge gates, and a stop
 * for a person at the step that reviews the change. */
const bug: WorkflowDiagramStep[] = [
  {
    id: "plan",
    label: "Plan the change",
    checks: [{ command: "plan_recorded" }],
    declarations: [{ label: "judge · 2 criteria" }],
    gate: "auto",
    gateLabel: "the checks decide, unless the Judge objects",
  },
  {
    id: "implement",
    label: "Implement",
    checks: [{ command: "every_manifest_check" }, { command: "diff_nonempty" }],
    declarations: [{ label: "judge · 3 criteria · gaming check" }],
    gate: "auto",
    gateLabel: "the checks decide, unless the Judge objects",
  },
  {
    id: "handoff",
    label: "Review the change",
    gate: "person",
    gateLabel: "a person answers",
  },
];

export const Bug: Story = {
  args: { steps: bug },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Plan the change")).toBeVisible();
    await expect(canvas.getByText("Implement")).toBeVisible();
    await expect(canvas.getByText("Review the change")).toBeVisible();
    await expect(canvas.getAllByText("the checks decide, unless the Judge objects")).toHaveLength(2);
    await expect(canvas.getByText("a person answers")).toBeVisible();
  },
};

/** `epic.json`: a loop back to the first step, capped at 5 passes, with a
 * person stopping the Job both before the split is dispatched and after the
 * wave rolls up. */
const epic: WorkflowDiagramStep[] = [
  {
    id: "plan",
    label: "Plan the wave",
    checks: [{ command: "artifact_exists" }, { command: "plan_recorded" }],
    declarations: [{ label: "judge · 2 criteria" }],
    gate: "person",
    gateLabel: "a person answers",
  },
  {
    id: "dispatch",
    label: "Dispatch the wave",
    checks: [{ command: "artifact_exists" }],
    gate: "auto",
  },
  {
    id: "roll_up",
    label: "Roll up the wave",
    checks: [{ command: "artifact_exists" }],
    gate: "person",
    gateLabel: "a person answers",
    loop: { to: "plan", label: "up to 5 passes" },
  },
];

export const Epic: Story = {
  args: { steps: epic },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Plan the wave")).toBeVisible();
    await expect(canvas.getByText("Roll up the wave")).toBeVisible();
    await expect(canvas.getByText("up to 5 passes")).toBeVisible();
  },
};

/** The lines a piece of a row's text is drawn on, measured with a range over
 * the text nodes that carry it. A flag broken in half draws on two. */
function linesOf(row: HTMLElement, text: string): number {
  const walker = document.createTreeWalker(row, NodeFilter.SHOW_TEXT);
  const nodes: Text[] = [];
  let whole = "";
  for (let node = walker.nextNode(); node !== null; node = walker.nextNode()) {
    nodes.push(node as Text);
    whole += node.textContent ?? "";
  }
  const from = whole.indexOf(text);
  if (from === -1) throw new Error(`${text} is not drawn in this row`);
  const range = document.createRange();
  range.setStart(...pointAt(nodes, from, false));
  range.setEnd(...pointAt(nodes, from + text.length, true));
  const tops = [...range.getClientRects()].filter((rect) => rect.width > 0).map((rect) => Math.round(rect.top));
  return new Set(tops).size;
}

/** Where an offset into that concatenated text lands, as a range boundary. At
 * a node's end it belongs to that node when it closes a range, and to the next
 * one when it opens one. */
function pointAt(nodes: Text[], offset: number, closing: boolean): [Text, number] {
  let seen = 0;
  for (const node of nodes) {
    const length = node.textContent?.length ?? 0;
    if (closing ? offset <= seen + length : offset < seen + length) return [node, offset - seen];
    seen += length;
  }
  const last = nodes.at(-1)!;
  return [last, last.textContent?.length ?? 0];
}

/** The row a piece of text is drawn in. */
function rowOf(canvasElement: HTMLElement, text: string): HTMLElement {
  const row = within(canvasElement)
    .getAllByRole("listitem")
    .find((candidate) => candidate.textContent?.includes(text));
  if (row === undefined) throw new Error(`no row draws ${text}`);
  return row;
}

/** The run column, the one width this diagram is ever drawn at. */
const inTheRunColumn: Decorator = (Story) => (
  <div style={{ width: "var(--w-run-column)" }}>
    <Story />
  </div>
);

const BUILD = "cargo_build · cargo build --workspace --locked";
const JUDGE = "judge · 3 criteria · gaming check";

/**
 * **The panel's own width, where a command has to wrap — and a flag is never
 * what breaks.** `--workspace` and `--locked` each sit on one line while the
 * command around them takes more than one, so this measures a wrap rather than
 * a line that happened to fit. Geometry, so it measures with a range: a
 * command broken after `--` reads identically through `getByText`.
 */
export const InTheRunColumn: Story = {
  args: {
    steps: [
      {
        id: "implement",
        label: "Implement",
        checks: [
          { command: BUILD },
          { command: "cargo_nextest · cargo nextest run --workspace", covers: "when crates/**" },
        ],
        declarations: [{ label: JUDGE }],
        gate: "auto",
        gateLabel: "the checks decide, unless the Judge objects",
        loop: { to: "implement", label: "up to 5 passes" },
      },
    ],
  },
  decorators: [inTheRunColumn],
  play: async ({ canvas, canvasElement }) => {
    const row = rowOf(canvasElement, BUILD);
    await expect(linesOf(row, BUILD)).toBeGreaterThan(1);
    await expect(linesOf(row, "--workspace")).toBe(1);
    await expect(linesOf(row, "--locked")).toBe(1);
    await expect(linesOf(rowOf(canvasElement, JUDGE), JUDGE)).toBe(1);
    await expect(canvas.getByText("Implement")).toBeVisible();
    await expect(canvas.getByText("the checks decide, unless the Judge objects")).toBeVisible();
    await expect(canvas.getByText("up to 5 passes")).toBeVisible();
  },
};

const LONG = "./scripts/verify-the-frozen-workflow-against-the-manifest-schema.sh";

/**
 * **One word wider than the box scrolls sideways, and takes nothing with it.**
 * There is no space in it to break at, so its row carries the scroll and the
 * box keeps the column's width. Clipping it instead would end a command
 * mid-path with nothing to say there was more.
 */
export const AWordWiderThanTheBox: Story = {
  args: {
    steps: [
      {
        id: "verify",
        label: "Verify",
        checks: [{ command: `schema · ${LONG}` }],
        gate: "person",
        gateLabel: "a person answers",
      },
    ],
  },
  decorators: [inTheRunColumn],
  play: async ({ canvasElement }) => {
    const row = rowOf(canvasElement, LONG);
    const column = canvasElement.firstElementChild!.getBoundingClientRect();
    await expect(linesOf(row, LONG)).toBe(1);
    await expect(row.scrollWidth).toBeGreaterThan(row.clientWidth);
    await expect(row.getBoundingClientRect().right).toBeLessThanOrEqual(column.right + 1);
    row.scrollLeft = row.scrollWidth;
    const drawn = document.createRange();
    drawn.selectNodeContents(row);
    await expect(drawn.getBoundingClientRect().right).toBeLessThanOrEqual(row.getBoundingClientRect().right + 1);
  },
};
