import type { Meta, StoryObj } from "@storybook/react-vite";
import { Absent } from "../absent";

/**
 * Bridge connects to a Fleet that has been running without it, and the list is
 * empty. Two readings, side by side, because a Fleet that is up with no work
 * and a Fleet that is not there are different states with different things to
 * do about them.
 *
 * **Both empty regions are `Board empty state`, and it is not built.** The
 * drawing names it as a bare `data-component`, which is its own convention for
 * "designed, no story yet". The copy it would carry is recorded in each
 * region rather than drawn here on the spot.
 */
const meta: Meta = {
  title: "Screens/First launch",
};
export default meta;

type Story = StoryObj;

export const FirstLaunch: Story = {
  render: () => (
    <div className="armada-screen">
      <div className="armada-screen__row">
        <div className="armada-screen__card" data-width="half">
          <span className="armada-screen__caption">Fleet running, no jobs</span>
          <div className="armada-screen__empty-slot">
            <Absent
              name="Board empty state"
              note={
                "Holds one line — “No jobs. Fleet has been up 6 days.” — and the New job " +
                "button beneath it. No centred glyph and no illustration."
              }
            />
          </div>
        </div>

        <div className="armada-screen__card" data-width="half">
          <span className="armada-screen__caption">Fleet is not running</span>
          <div className="armada-screen__empty-slot">
            <Absent
              name="Board empty state"
              note={
                "Holds “Fleet is not running. Bridge has nothing to read.”, then " +
                "armada-fleet start as a mono value to copy rather than a button, then " +
                "“Run that in a terminal. Bridge connects on its own once the runtime " +
                "file appears.”"
              }
            />
          </div>
        </div>
      </div>
    </div>
  ),
};
