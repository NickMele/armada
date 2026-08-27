import type { Meta, StoryObj } from "@storybook/react-vite";
import { DispatchAJobFullWithTheM1SubsetMarked } from "./DispatchAJobFullWithTheM1SubsetMarked";

/**
 * Journey · Dispatch a Job. What M1 renders where the approval card goes.
 *
 * **The full approval card is not here, and that is the design.** The drawing
 * puts it beside the composer as the dimmed half of a two-up — the full design,
 * deliberately not built at this milestone, because the three glance fields the
 * card exists for are a diff size, a job type and an estimated cost, none of
 * which Armada can measure before a drone has run. A region held open for it
 * would be a hole for something the milestone does not render, so the screen
 * renders the composer alone.
 *
 * **Approve lands the job in `queued`, not `running`**, and **Cancel writes
 * `killed`** — a job you never dispatched was not stopped, it was abandoned.
 */
const meta: Meta<typeof DispatchAJobFullWithTheM1SubsetMarked> = {
  title: "Screens/Dispatch a job — full, with the M1 subset marked",
  component: DispatchAJobFullWithTheM1SubsetMarked,
};
export default meta;

type Story = StoryObj<typeof DispatchAJobFullWithTheM1SubsetMarked>;

export const Dispatch: Story = {
  render: () => (
    <div className="armada-screen">
      <DispatchAJobFullWithTheM1SubsetMarked
        composer={{
          title: "Coalesce concurrent token refreshes",
          brief:
            "A burst of 401s should produce one refresh call, not one per request. Keep the retry ceiling where it is.",
          workflows: <option>bug — 4 steps</option>,
          project: "armada",
          glance: [
            { label: "Steps", value: "4 · 2 gated" },
            { label: "Checks", value: "build, test" },
          ],
          provenance: "Dispatched by you",
        }}
      />
    </div>
  ),
};
