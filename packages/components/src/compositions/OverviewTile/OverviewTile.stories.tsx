import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
import { OverviewTile, OverviewTileBand } from "./OverviewTile";

/**
 * One story per shape a tile takes: not read yet, a reading in each borrowed
 * hue, one that opens something and one that opens nothing. The five readings
 * are drawn from wire data in `Screens/Overview tiles`.
 */
const meta: Meta<typeof OverviewTile> = {
  title: "Compositions/Overview tile",
  component: OverviewTile,
  decorators: [
    (Story) => (
      <OverviewTileBand>
        <Story />
      </OverviewTileBand>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof OverviewTile>;

/** Not read yet: a placeholder where the reading lands, never a zero. */
export const NotReadYet: Story = {
  args: { label: "Doctor" },
};

/** A reading that opens nothing is not a control, and trails no glyph. */
export const OpensNothing: Story = {
  args: {
    label: "Fleet",
    value: "Fleet running",
    tone: "completed-success",
    detail: "pid 4242 · port 7878",
    detailFace: "mono",
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("group", { name: "Fleet" })).toBeVisible();
    await expect(canvas.queryByRole("button")).toBeNull();
  },
};

/** One press from the surface the reading belongs to. */
export const Opens: Story = {
  args: {
    label: "Drones",
    value: "2 of 4",
    valueFace: "mono",
    detail: "2 free",
    opens: "Fleet settings",
    onOpen: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: /Open Fleet settings/ }));
    await expect(args.onOpen).toHaveBeenCalledOnce();
  },
};

/** A word Doctor reported, in the Job value it borrows. */
export const Warn: Story = {
  args: { label: "Doctor", value: "warn", valueFace: "mono", tone: "awaiting-review", detail: "Manifest: warn" },
};

export const Fail: Story = {
  args: { label: "Doctor", value: "fail", valueFace: "mono", tone: "completed-failed", detail: "SQLite: fail" },
};

/** Behind rather than broken, so caution and plain sans. */
export const Caution: Story = {
  args: {
    label: "Manifest drift",
    value: "2 behind",
    tone: "notice-caution",
    detail: "2 of 3 repositories",
    opens: "Manifest",
    onOpen: fn(),
  },
};
