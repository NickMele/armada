import type { Meta, StoryObj } from "@storybook/react-vite";
import { AttachmentChip } from "./AttachmentChip";

/**
 * One story per state this component actually has: named-and-removable is
 * the ordinary case on a draft brief, a long name is the one geometry has to
 * survive, and read-only is what a chip renders once `onRemove` is left out.
 * No contract entry named this component before it existed — see the report
 * this shipped beside, and the doc comment on `AttachmentChip.tsx`.
 */
const meta: Meta<typeof AttachmentChip> = {
  title: "Primitives/AttachmentChip",
  component: AttachmentChip,
};
export default meta;

type Story = StoryObj<typeof AttachmentChip>;

export const Default: Story = {
  args: { filename: "screenshot.png", onRemove: () => {} },
};

export const LongFilename: Story = {
  args: {
    filename: "a-very-long-filename-that-should-truncate-rather-than-widen-the-row.png",
    onRemove: () => {},
  },
};

/** No `onRemove` — a chip with nothing to take back. */
export const ReadOnly: Story = {
  args: { filename: "evidence.log" },
};

/** An address attached beside the request, rather than a file staged from disk. */
export const ALink: Story = {
  args: { filename: "armada/1162", kind: "link", onRemove: () => {} },
};

/** One node of a Studio, carried into the request it was dispatched from. */
export const AStudioNode: Story = {
  args: { filename: "The Drones stat says nothing", kind: "node", onRemove: () => {} },
};

/**
 * A picture drawn beside the prompt, with where it was made read first —
 * #1547. Read-only: a sketch is taken back on the pad, where the boxes going
 * are visible, rather than by a press on a chip that says only `sketch 1`.
 */
export const ASketch: Story = {
  args: { filename: "sketch 1", from: "From a Studio" },
};

/** The same picture drawn from nothing, which says so by naming no source. */
export const ASketchFromNothing: Story = {
  args: { filename: "sketch 1" },
};
