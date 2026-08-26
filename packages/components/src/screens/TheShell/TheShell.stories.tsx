import type { Meta, StoryObj } from "@storybook/react-vite";
import { Activity } from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import { Select } from "../../primitives/Select/Select";
import { Sidebar } from "../../compositions/Sidebar/Sidebar";
import { StatusBar } from "../../compositions/StatusBar/StatusBar";

/**
 * Rail, panel, status bar — with one surface in the rail, because one surface
 * exists.
 *
 * A screen is an arrangement of components around fixture data, so there is
 * nothing here to export and nothing to import. The values are the drawing's
 * own: pid 4417, port 7411, one drone, six jobs, one of them waiting on you.
 */
const meta: Meta = {
  title: "Screens/The shell",
};
export default meta;

type Story = StoryObj;

export const Shell: Story = {
  render: () => (
    <div className="armada-screen">
      <div className="armada-screen__window">
        <Sidebar
          appName="Armada"
          sectionLabel="Bridge"
          activeId="active"
          header={
            <Select aria-label="Project">
              <option>armada</option>
            </Select>
          }
          surfaces={[
            // The drawing's rail row carries a label and a count and no glyph.
            // Sidebar requires one, and `activity` is what the registry assigns
            // to Active jobs, so that is the glyph. Reported.
            { id: "active", label: "Active jobs", icon: Activity, count: 6 },
          ]}
        />
        <div className="armada-screen__panel">
          <div className="armada-screen__panel-head">
            <div className="armada-screen__titles">
              <span className="armada-screen__title">Active jobs</span>
              <span className="armada-screen__summary">6 jobs. 1 awaiting approval.</span>
            </div>
            <Button variant="primary">New job</Button>
          </div>
          <div className="armada-screen__mount">The list mounts here — 1d</div>
          <StatusBar
            fleet="running"
            fleetLabel="Fleet running"
            detail="pid 4417 · port 7411 · 1 drone"
            spend="today ~$4.80"
          />
        </div>
      </div>

      <span className="armada-screen__eyebrow">
        The status bar says Fleet out loud — three states
      </span>
      <div className="armada-screen__col">
        <div className="armada-screen__bar-frame">
          <StatusBar
            fleet="running"
            fleetLabel="Fleet running"
            detail="pid 4417 · port 7411 · 1 drone"
            spend="today ~$4.80"
          />
        </div>
        <div className="armada-screen__bar-frame">
          <StatusBar
            fleet="not-running"
            fleetLabel="Fleet is not running"
            detail="no runtime file at ~/.armada/fleet.json"
            advice="Start it from the terminal."
          />
        </div>
        <div className="armada-screen__bar-frame">
          <StatusBar
            fleet="unreachable"
            fleetLabel="Fleet unreachable"
            detail="pid 4417 alive on port 7411 · no response for 20s"
            advice="The last job state read is 20s old."
          />
        </div>
      </div>
    </div>
  ),
};
