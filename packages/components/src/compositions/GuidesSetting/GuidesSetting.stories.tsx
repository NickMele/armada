import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import { useState } from "react";

import { Card, CardContent, CardHeader, CardTitle } from "../../primitives/Card/Card";
import { GuidanceProvider } from "../../guidance";
import { GUIDE_COMPLETION } from "../../guides";
import { GuideMark } from "../GuideMark/GuideMark";
import { GuidesSetting } from "./GuidesSetting";

const meta: Meta<typeof GuidesSetting> = {
  title: "Compositions/Guides setting",
  component: GuidesSetting,
  decorators: [
    (Story) => (
      <GuidanceProvider remembered={false}>
        <div style={{ padding: "var(--space-6)", background: "var(--surface-canvas)", maxWidth: "var(--w-dialog-wide)" }}>
          <Card>
            <CardHeader>
              <CardTitle>Guides</CardTitle>
            </CardHeader>
            <CardContent>
              <Story />
            </CardContent>
          </Card>
        </div>
      </GuidanceProvider>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof GuidesSetting>;

/** On, which is where a new window starts. */
export const AsSettingsDrawsIt: Story = {
  args: { onReadGuides: fn() },
};

/** No catalogue to send anybody to. The switch is the whole card. */
export const WithNowhereToGo: Story = {};

/**
 * **The switch turns all of them off.** Turned off here, a piece nobody has
 * met stops opening itself — which is the claim, and it cannot be read off the
 * switch's own state.
 */
export const TurnsThemAllOff: Story = {
  render: () => (
    <>
      <GuidesSetting />
      <Later />
    </>
  ),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("switch"));

    // A piece arrives that nobody has met. With first contact off, nothing
    // opens — and the `?` is still there to ask with.
    await userEvent.click(canvas.getByRole("button", { name: "A piece arrives" }));
    await expect(within(document.body).queryAllByRole("dialog")).toHaveLength(0);
    await expect(
      canvas.getByRole("button", { name: `Open guide ${GUIDE_COMPLETION.number}, ${GUIDE_COMPLETION.title}` }),
    ).toBeVisible();
  },
};

/**
 * A piece nobody has met, mounted on a press rather than on load — so the
 * switch is already answered by the time the piece is on screen.
 */
function Later() {
  const [arrived, setArrived] = useState(false);
  return (
    <>
      <button type="button" onClick={() => setArrived(true)}>
        A piece arrives
      </button>
      {arrived ? <GuideMark guide={GUIDE_COMPLETION} /> : null}
    </>
  );
}
