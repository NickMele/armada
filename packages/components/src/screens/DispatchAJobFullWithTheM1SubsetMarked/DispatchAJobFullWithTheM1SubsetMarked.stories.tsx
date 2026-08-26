import type { Meta, StoryObj } from "@storybook/react-vite";
import { Absent } from "../absent";

/**
 * Journey · Dispatch a Job. The full approval card and M1's reduced version of
 * it, side by side.
 *
 * **Both columns are `Approval card` and `Job composer`, and neither is
 * built.** The drawing names both as bare `data-component` values. Their fields
 * are all primitives that exist — Input, Textarea, Select, Button — but the card
 * that arranges them is the thing being agreed, so the region is named rather
 * than assembled on the spot.
 */
const meta: Meta = {
  title: "Screens/Dispatch a job — full, with the M1 subset marked",
};
export default meta;

type Story = StoryObj;

export const Dispatch: Story = {
  render: () => (
    <div className="armada-screen">
      <div className="armada-screen__row">
        <div className="armada-screen__card" data-bright data-width="card">
          <span className="armada-screen__caption">
            The approval card — the full design
          </span>
          <div className="armada-screen__slot">
            <Absent
              name="Approval card"
              note={
                "Holds the title “Coalesce concurrent token refreshes”, its brief, then " +
                "the three glance fields the card exists for — Diff size ~4 files, Job " +
                "type feature, Cost, estimated ~$3.20 of $20 — then Workflow bug, 4 " +
                "steps · Workspace armada · Criteria 4, then Cancel beside Approve and " +
                "dispatch."
              }
            />
          </div>
        </div>

        <div className="armada-screen__card" data-bright data-width="card">
          <span className="armada-screen__caption">What M1 renders</span>
          <div className="armada-screen__slot">
            <Absent
              name="Job composer"
              note={
                "Holds a Title input, a Brief textarea, a Workflow select reading “bug — " +
                "4 steps” beside a read-only Project armada, then the two-up glance strip " +
                "Steps 4 · 2 gated and Checks build, test, then Cancel beside Approve and " +
                "dispatch — the one accent fill in the whole milestone."
              }
            />
          </div>
        </div>
      </div>
    </div>
  ),
};
