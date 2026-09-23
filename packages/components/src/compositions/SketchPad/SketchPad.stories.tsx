import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, waitFor } from "storybook/test";

import {
  SketchPad,
  type SketchBox,
  type SketchLine,
  type SketchPoint,
  type SketchStroke,
} from "./SketchPad";

const meta: Meta<typeof SketchPad> = {
  title: "Compositions/Sketch pad",
  component: SketchPad,
  parameters: { layout: "padded" },
};
export default meta;

type Story = StoryObj<typeof SketchPad>;

/** What the arc's person drew: the stat, the panel it opens, and the read behind it. */
const BOXES: SketchBox[] = [
  { id: "b1", x: 0, y: 0, body: "Drones 1 of 2" },
  { id: "b2", x: 0, y: 160, body: "a panel under it, one Drone to a line" },
  { id: "b3", x: 300, y: 160, body: "the Job and the step each Drone is on" },
];

const LINES: SketchLine[] = [
  { id: "b1-b2", from: "b1", to: "b2" },
  { id: "b2-b3", from: "b2", to: "b3" },
];

/** A ring round the two boxes the panel is made of — what no box and no join says. */
const STROKES: SketchStroke[] = [
  {
    id: "s1",
    points: [
      { x: -40, y: 126 },
      { x: 280, y: 108 },
      { x: 570, y: 132 },
      { x: 588, y: 214 },
      { x: 280, y: 248 },
      { x: -30, y: 230 },
      { x: -46, y: 160 },
      { x: -40, y: 126 },
    ],
  },
];

const SAID = "The panel opens under the stat, with the Drone's Job on the first line.";

/**
 * The pad, with a caller holding what is on it.
 *
 * **Every edit is the caller's**, the way `DispatchJob` holds the drawing: the
 * pad reports and draws, and a story that let it keep its own boxes would be
 * proving something the app does not do.
 */
function Held({
  boxes: open,
  lines: drawn,
  strokes: inked,
  ...rest
}: Parameters<typeof SketchPad>[0]) {
  const [boxes, setBoxes] = useState<readonly SketchBox[]>(open);
  const [lines, setLines] = useState<readonly SketchLine[]>(drawn);
  const [strokes, setStrokes] = useState<readonly SketchStroke[]>(inked);
  const [said, setSaid] = useState(rest.said);

  return (
    <SketchPad
      {...rest}
      boxes={boxes}
      lines={lines}
      strokes={strokes}
      onDraw={(points: readonly SketchPoint[]) =>
        setStrokes((current) => [...current, { id: `s${String(current.length + 1)}`, points }])
      }
      onUndo={() => setStrokes((current) => current.slice(0, -1))}
      said={said}
      onSaid={setSaid}
      onAdd={(at) =>
        setBoxes((current) => [
          ...current,
          { id: `b${String(current.length + 1)}`, x: at.x, y: at.y, body: "" },
        ])
      }
      onBody={(id, body) =>
        setBoxes((current) => current.map((box) => (box.id === id ? { ...box, body } : box)))
      }
      onMove={(id, at) =>
        setBoxes((current) => current.map((box) => (box.id === id ? { ...box, ...at } : box)))
      }
      onRemove={(ids) => {
        setBoxes((current) => current.filter((box) => !ids.includes(box.id)));
        setLines((current) =>
          current.filter((line) => !ids.includes(line.from) && !ids.includes(line.to)),
        );
      }}
      onJoin={(from, to) =>
        setLines((current) => [...current, { id: `${from}-${to}`, from, to }])
      }
    />
  );
}

const args = {
  label: "What the Drones stat should open into",
  boxes: BOXES,
  lines: LINES,
  strokes: STROKES,
  said: SAID,
  from: "rail-stats",
  onMove: () => undefined,
  onBody: () => undefined,
  onAdd: () => undefined,
  onRemove: () => undefined,
  onJoin: () => undefined,
  onDraw: () => undefined,
  onUndo: () => undefined,
  onSaid: () => undefined,
};

/**
 * A picture already drawn, made from a Studio node: boxes, the joins between
 * them, and a ring round the two the panel is made of.
 */
export const Drawn: Story = { args, render: (props) => <Held {...props} /> };

/** The boxes on their own, which is every pad drawn before the pen existed. */
export const NoHand: Story = {
  args: { ...args, strokes: [] },
  render: (props) => <Held {...props} />,
};

/**
 * A pad nobody has drawn on. **The note is the whole state** — a blank canvas
 * under the controls says nothing about what it is for.
 */
export const Nothing: Story = {
  args: { ...args, boxes: [], lines: [], strokes: [], said: "", from: undefined },
  render: (props) => <Held {...props} />,
};

/** Nothing may be drawn while the connection is not live. */
export const NotLive: Story = {
  args: { ...args, disabled: true },
  render: (props) => <Held {...props} />,
};

/**
 * The box acts, which no still can show.
 *
 * **Join is the one worth asserting.** It takes exactly two boxes, so the
 * control is off until two are picked and the reason is on it — a dead control
 * with no reason reads as broken.
 */
export const Drawing: Story = {
  args: { ...args, boxes: [], lines: [], strokes: [], said: "" },
  render: (props) => <Held {...props} />,
  play: async ({ canvas, userEvent, step }) => {
    const add = canvas.getByRole("button", { name: "Add a box" });
    const join = canvas.getByRole("button", { name: "Join" });
    const remove = canvas.getByRole("button", { name: "Remove" });

    await step("nothing is picked, so neither act is offered and both say why", async () => {
      await expect(join).toBeDisabled();
      await expect(join).toHaveAttribute("title", "Pick two boxes to join them.");
      await expect(remove).toBeDisabled();
    });

    await step("a box added is empty, and writing in it names it", async () => {
      await userEvent.click(add);
      const box = await canvas.findByRole("group", { name: "An empty box" });
      await expect(box).toBeVisible();
      await userEvent.type(canvas.getByRole("textbox", { name: "The words in this box" }), "the stat");
      await waitFor(() =>
        expect(canvas.getByRole("group", { name: "Box: the stat" })).toBeVisible(),
      );
    });

    await step("one box picked offers Remove but not Join", async () => {
      await userEvent.click(canvas.getByRole("group", { name: "Box: the stat" }));
      await waitFor(() => expect(remove).toBeEnabled());
      await expect(join).toBeDisabled();
    });

    await step("two boxes picked draw a line between them", async () => {
      await userEvent.click(add);
      const second = await canvas.findByRole("group", { name: "An empty box" });
      await userEvent.click(second);
      await userEvent.keyboard("{Meta>}");
      await userEvent.click(canvas.getByRole("group", { name: "Box: the stat" }));
      await userEvent.keyboard("{/Meta}");
      await waitFor(() => expect(join).toBeEnabled());
      await userEvent.click(join);
      await waitFor(() => expect(canvas.getAllByRole("group", { name: /^A line from / })).toHaveLength(1));
    });

    await step("a box taken off takes the line that hung on it", async () => {
      await userEvent.click(canvas.getByRole("group", { name: "Box: the stat" }));
      await waitFor(() => expect(remove).toBeEnabled());
      await userEvent.click(remove);
      await waitFor(() =>
        expect(canvas.queryByRole("group", { name: "Box: the stat" })).toBeNull(),
      );
      await expect(canvas.queryAllByRole("group", { name: /^A line from / })).toHaveLength(0);
    });
  },
};

/** What a hand's line is called, and what a still cannot show it doing. */
const A_HAND_LINE = "Drawn by hand";

/**
 * One drag across the pad, in pointer events.
 *
 * **Dispatched by hand, and the layer found by what is under the point.** The
 * layer that catches the pen carries no role — a surface to draw on is not a
 * control — so there is nothing to locate it by, and `elementFromPoint` finds
 * what a real pointer would have hit. The move and the lift go to the window,
 * where the pad listens, so a line drawn off the pad still ends.
 */
function drag(graph: HTMLElement, through: readonly SketchPoint[]): void {
  const at = graph.getBoundingClientRect();
  const place = (point: SketchPoint) => ({ x: at.x + point.x, y: at.y + point.y });
  const start = place(through[0]!);
  const pen = document.elementFromPoint(start.x, start.y);
  if (pen === null) throw new Error("Nothing is under the pen.");
  const event = (kind: string, where: { x: number; y: number }) =>
    new PointerEvent(kind, { clientX: where.x, clientY: where.y, bubbles: true, button: 0 });

  pen.dispatchEvent(event("pointerdown", start));
  for (const point of through.slice(1)) window.dispatchEvent(event("pointermove", place(point)));
  window.dispatchEvent(event("pointerup", place(through[through.length - 1]!)));
}

/**
 * The pen: a line drawn by hand, and the undo that takes it back.
 *
 * **The reason this is a `play` and not a still.** A stroke exists only after a
 * drag, so no rendering can show that dragging makes one; Undo's scope — the
 * last line and nothing else — is a rule about what does *not* happen; and a
 * press with no travel making nothing is the defect a thinned freehand line
 * invites, an invisible stroke sitting in the picture.
 */
export const ByHand: Story = {
  args: { ...args, boxes: [], lines: [], strokes: [], said: "", from: undefined },
  render: (props) => <Held {...props} />,
  play: async ({ canvas, userEvent, step }) => {
    const draw = canvas.getByRole("button", { name: "Draw" });
    const undo = canvas.getByRole("button", { name: "Undo" });
    const graph = canvas.getByLabelText(args.label);

    await step("nothing is drawn by hand, so Undo is off and says so", async () => {
      await expect(draw).toHaveAttribute("aria-pressed", "false");
      await expect(undo).toBeDisabled();
      await expect(undo).toHaveAttribute("title", "Nothing has been drawn by hand.");
    });

    await step("the pen says it is down", async () => {
      await userEvent.click(draw);
      await waitFor(() => expect(draw).toHaveAttribute("aria-pressed", "true"));
    });

    await step("a drag leaves a line, and Undo comes alive", async () => {
      drag(graph, [
        { x: 60, y: 60 },
        { x: 110, y: 90 },
        { x: 160, y: 60 },
        { x: 210, y: 120 },
      ]);
      await waitFor(() => expect(canvas.getAllByRole("img", { name: A_HAND_LINE })).toHaveLength(1));
      await expect(undo).toBeEnabled();
    });

    await step("a press that goes nowhere is not a line", async () => {
      drag(graph, [{ x: 240, y: 200 }]);
      await expect(canvas.getAllByRole("img", { name: A_HAND_LINE })).toHaveLength(1);
    });

    await step("a second line goes on top, and Undo takes back only the last", async () => {
      drag(graph, [
        { x: 80, y: 190 },
        { x: 150, y: 190 },
        { x: 220, y: 175 },
      ]);
      await waitFor(() => expect(canvas.getAllByRole("img", { name: A_HAND_LINE })).toHaveLength(2));
      await userEvent.click(undo);
      await waitFor(() => expect(canvas.getAllByRole("img", { name: A_HAND_LINE })).toHaveLength(1));
      await expect(undo).toBeEnabled();
    });

    await step("the last line back leaves the pad empty and Undo off again", async () => {
      await userEvent.click(undo);
      await waitFor(() => expect(canvas.queryAllByRole("img", { name: A_HAND_LINE })).toHaveLength(0));
      await expect(undo).toBeDisabled();
    });

    await step("any other act puts the pen down", async () => {
      // The pen has been down throughout: nothing above it puts it away.
      await expect(draw).toHaveAttribute("aria-pressed", "true");
      await userEvent.click(canvas.getByRole("button", { name: "Add a box" }));
      await waitFor(() => expect(draw).toHaveAttribute("aria-pressed", "false"));
    });
  },
};
