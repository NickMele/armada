import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, waitFor } from "storybook/test";

import { SketchPad, type SketchBox, type SketchLine } from "./SketchPad";

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

const SAID = "The panel opens under the stat, with the Drone's Job on the first line.";

/**
 * The pad, with a caller holding what is on it.
 *
 * **Every edit is the caller's**, the way `DispatchJob` holds the drawing: the
 * pad reports and draws, and a story that let it keep its own boxes would be
 * proving something the app does not do.
 */
function Held({ boxes: open, lines: drawn, ...rest }: Parameters<typeof SketchPad>[0]) {
  const [boxes, setBoxes] = useState<readonly SketchBox[]>(open);
  const [lines, setLines] = useState<readonly SketchLine[]>(drawn);
  const [said, setSaid] = useState(rest.said);

  return (
    <SketchPad
      {...rest}
      boxes={boxes}
      lines={lines}
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
  said: SAID,
  from: "rail-stats",
  onMove: () => undefined,
  onBody: () => undefined,
  onAdd: () => undefined,
  onRemove: () => undefined,
  onJoin: () => undefined,
  onSaid: () => undefined,
};

/** A picture already drawn, made from a Studio node. */
export const Drawn: Story = { args, render: (props) => <Held {...props} /> };

/**
 * A pad nobody has drawn on. **The note is the whole state** — a blank canvas
 * under three controls says nothing about what it is for.
 */
export const Nothing: Story = {
  args: { ...args, boxes: [], lines: [], said: "", from: undefined },
  render: (props) => <Held {...props} />,
};

/** Nothing may be drawn while the connection is not live. */
export const NotLive: Story = {
  args: { ...args, disabled: true },
  render: (props) => <Held {...props} />,
};

/**
 * The three acts, which no still can show.
 *
 * **Join is the one worth asserting.** It takes exactly two boxes, so the
 * control is off until two are picked and the reason is on it — a dead control
 * with no reason reads as broken.
 */
export const Drawing: Story = {
  args: { ...args, boxes: [], lines: [], said: "" },
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
