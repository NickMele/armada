import type { Meta, StoryObj } from "@storybook/react-vite";
import { CircleDot } from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import type { DroneTurn } from "../../compositions/DroneTurns/DroneTurns";
import { WatchingADroneWork } from "./WatchingADroneWork";

/**
 * Watching a drone work — the read-only pane, in each of the states the socket
 * can put it in.
 *
 * **The only control on the screen leaves it.** Observing takes nothing over:
 * the Drone is not told, the Job's status does not move and no transition is
 * recorded, so a control that ended, redirected or answered the Drone would be
 * Pilot wearing this screen's clothes.
 *
 * The header verb comes from the enum→verb map, written into the fixture
 * because a story has no generated module to read. Bridge reads one.
 */
const meta: Meta<typeof WatchingADroneWork> = {
  title: "Screens/Watching a drone work",
  component: WatchingADroneWork,
};
export default meta;

type Story = StoryObj<typeof WatchingADroneWork>;

const NOTHING_YET = "This job has no turns. It was never dispatched, so no drone has written one.";

const heading = {
  status: "running",
  statusIcon: CircleDot,
  statusLabel: "Running",
  headline: "Split the settings reducer",
  jobId: "job_2d90bb",
  fields: [
    { label: "Step", value: "2 of 4", mono: true },
    { label: "Drone", value: "drone_9c41", mono: true, copyValue: "drone_9c41" },
    { label: "Elapsed", value: "11m 03s", mono: true },
  ],
  // Leaves the view. Never an act on the Drone.
  actions: <Button variant="ghost">Back to the job</Button>,
};

const turns: DroneTurn[] = [
  {
    id: "1",
    at: "09:14:02",
    kind: "started",
    // The model is whatever the Job named. A vendor spelling belongs in
    // `adapters` and nowhere else, so the fixture carries a placeholder.
    subject: "sess_01JB4 · the job's model · 2 mcp servers",
  },
  {
    id: "2",
    at: "09:14:03",
    kind: "said",
    said: "Reading the settings module before I split anything, so the public signature survives.",
  },
  { id: "3", at: "09:14:04", kind: "called", subject: "Read · call_7f21", answer: "Answered." },
  {
    id: "4",
    at: "09:14:11",
    kind: "called",
    subject: "Edit · call_7f23",
    answer: "No answer yet.",
  },
];

/** A Drone is writing now. The history, then the live rows, on one connection. */
export const ADroneWriting: Story = {
  render: () => (
    <div className="armada-screen">
      <WatchingADroneWork heading={heading} turns={turns} emptyNote={NOTHING_YET} live />
    </div>
  ),
};

/**
 * A Job that was never dispatched. `opened` says nothing is live, the history
 * is empty and the socket closes saying why. **Ordinary, not a failure** — so
 * the pane says what is true rather than drawing an error.
 */
export const AJobWithNoTranscript: Story = {
  render: () => (
    <div className="armada-screen">
      <WatchingADroneWork
        heading={{ ...heading, statusLabel: "Needs approval", status: "awaiting-approval" }}
        turns={[]}
        emptyNote={NOTHING_YET}
        closedBecause="nothing_writing"
      />
    </div>
  ),
};

/**
 * A viewer that fell behind. The per-Job channel is drop-oldest, so the loss is
 * this subscription's rather than the sink's — and it is counted and said,
 * because a history with a silent hole reads as a Drone that went quiet.
 */
export const AViewerThatMissedRows: Story = {
  render: () => (
    <div className="armada-screen">
      <WatchingADroneWork
        heading={heading}
        turns={turns}
        emptyNote={NOTHING_YET}
        live
        missed={34}
      />
    </div>
  ),
};

/**
 * A Drone that outlived the Fleet that spawned it. Fleet's writer does not
 * reattach, so the history is whole and nothing is live.
 */
export const ADroneThatOutlivedItsFleet: Story = {
  render: () => (
    <div className="armada-screen">
      <WatchingADroneWork
        heading={heading}
        turns={turns}
        emptyNote={NOTHING_YET}
        skipped={128}
        closedBecause="drone_ended"
      />
    </div>
  ),
};

/**
 * The pane could not be opened. **A view that cannot open is an error, not a
 * health state**, and it reaches a person at the moment they asked — which is
 * why it is not the same rendering as a Job with no turns.
 */
export const TheTurnsCouldNotBeRead: Story = {
  render: () => (
    <div className="armada-screen">
      <WatchingADroneWork
        heading={heading}
        turns={[]}
        emptyNote={NOTHING_YET}
        failure="Fleet did not answer on this job's observe socket: connect ECONNREFUSED 127.0.0.1:7777"
      />
    </div>
  ),
};
