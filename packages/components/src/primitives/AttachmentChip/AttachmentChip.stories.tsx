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
