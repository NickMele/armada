import type { Meta, StoryObj } from "@storybook/react-vite";
import { StatusBar } from "./StatusBar";

/**
 * One story per state the contract names: Fleet's three readings, the two
 * counts, and the bar in both billing modes.
 *
 * The three Fleet sentences are fixed copy from `docs/contracts/
 * design-system.md`. Fleet's own strings are identical every time, because
 * uniformity is scannability.
 */
const meta: Meta<typeof StatusBar> = {
  title: "Compositions/StatusBar",
  component: StatusBar,
};
export default meta;

type Story = StoryObj<typeof StatusBar>;

/** Idle. The assertion is never collapsed, only the detail around it. */
export const FleetRunningIdle: Story = {
  args: { fleet: "running", fleetLabel: "Fleet running" },
};

/** Working, on a personal machine: the gating figure is the quota floor. */
export const FleetRunningPersonalMachine: Story = {
  args: {
    fleet: "running",
    fleetLabel: "Fleet running",
    detail: "pid 4417 · port 7411 · 1 drone",
    items: ["3 jobs"],
    spend: "68% quota left",
  },
};

/**
 * Working, on a work machine: the gating figure is the dollar cap, marked
 * approximate because a derived figure is not a measured one.
 */
export const FleetRunningWorkMachine: Story = {
  args: {
    fleet: "running",
    fleetLabel: "Fleet running",
    detail: "pid 4417 · port 7411 · 1 drone",
    items: ["3 jobs"],
    spend: "~$2.40 of $20",
  },
};

/**
 * No runtime file: Fleet is not there. Bridge cannot start it, so the bar says
 * what to do. This is the state a person actually meets at M1, since Fleet is
 * started by hand — and whether it should read the same during onboarding is
 * open, `[status-bar-onboarding]`.
 */
export const FleetNotRunning: Story = {
  args: {
    fleet: "not-running",
    fleetLabel: "Fleet is not running",
    detail: "no runtime file at ~/.armada/fleet.json",
    advice: "Start it from the terminal.",
  },
};

/**
 * A live pid that does not answer: Fleet is wedged. A different thing to do
 * about it, so a different sentence — and how stale the last read is, because
 * that is what a person decides on.
 */
export const FleetUnreachable: Story = {
  args: {
    fleet: "unreachable",
    fleetLabel: "Fleet unreachable",
    detail: "pid 4417 alive on port 7411 · no response for 20s",
    advice: "The last job state read is 20s old.",
  },
};

/**
 * Both counts, non-zero. They are the only status colour in the bar besides
 * the dot. How much louder than its own token an escalation count renders is
 * open — `[status-bar-loudness]` — so both take their token and nothing more,
 * which is the least that can be said without pre-empting the answer.
 */
export const WithBothCounts: Story = {
  args: {
    fleet: "running",
    fleetLabel: "Fleet running",
    items: ["3 jobs"],
    escalations: 2,
    approvals: 4,
    spend: "68% quota left",
  },
};

/** One of each. The plural is inflected, and the noun is not sanctioned copy. */
export const WithOneOfEach: Story = {
  args: {
    fleet: "running",
    fleetLabel: "Fleet running",
    items: ["1 job"],
    escalations: 1,
    approvals: 1,
  },
};

/**
 * Escalations only. Approvals never appear at zero, which is what makes a
 * non-zero one worth reading.
 */
export const WithEscalationsOnly: Story = {
  args: {
    fleet: "running",
    fleetLabel: "Fleet running",
    items: ["3 jobs"],
    escalations: 2,
    spend: "~$2.40 of $20",
  },
};

/** Five items is the ceiling, and this is it. */
export const AtTheItemCeiling: Story = {
  args: {
    fleet: "running",
    fleetLabel: "Fleet running",
    detail: "pid 4417 · port 7411 · 1 drone",
    items: ["3 jobs"],
    escalations: 2,
    spend: "~$2.40 of $20",
  },
};
