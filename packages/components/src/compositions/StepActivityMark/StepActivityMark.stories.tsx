import type { Meta, StoryObj } from "@storybook/react-vite";
import { StepActivityMark, type StepActivity } from "./StepActivityMark";

/**
 * One story per step-activity value the contract and the registry name
 * together. Six are `job_steps.state` values, `failed` renders `last_verdict`,
 * and `killed` is named by the design system as the value that must take no
 * hue.
 *
 * The mark is 16px wide on purpose, so a numbered row and a glyphed row share
 * one left edge in a rail.
 */
const meta: Meta<typeof StepActivityMark> = {
  title: "Compositions/Step activity mark",
  component: StepActivityMark,
};
export default meta;

type Story = StoryObj<typeof StepActivityMark>;

/**
 * A step that has not run shows its own number rather than a glyph. There is
 * no `not_started` entry in `packages/icons/icons.toml` and no borrowing for
 * it, and the M1 drawing answers why: a position is the only fact a step that
 * has not run carries.
 */
export const NotStarted: Story = {
  args: { activity: "not_started", label: "Not started", ordinal: 3 },
};

/**
 * `not_started` with no ordinal supplied renders an empty slot — the visible
 * shape of a registry with no glyph for this value. Reported.
 */
export const NotStartedWithNoOrdinal: Story = {
  args: { activity: "not_started", label: "Not started" },
};

export const Running: Story = {
  args: { activity: "running", label: "Running" },
};

/**
 * The rail's current step, on the Job being read. The inner dot of `circle-dot`
 * moves on opacity and scale at `--duration-pulse`; the ring holds still.
 */
export const RunningPulsing: Story = {
  args: { activity: "running", label: "Running", pulsing: true },
};

/**
 * `eye`, not `clock`. The registry's borrowing convention lists `eye`, and the
 * hue agrees — `--step-waiting` aliases `--status-awaiting-review`, whose badge
 * is `eye`. `docs/contracts/iconography.md` says `clock` in prose. Reported.
 */
export const AwaitingHuman: Story = {
  args: { activity: "awaiting_human", label: "Waiting on you" },
};

/** Retrying takes no hue. It is a position, not an outcome. */
export const Retrying: Story = {
  args: { activity: "retrying", label: "Retrying" },
};

export const Advanced: Story = {
  args: { activity: "advanced", label: "Advanced" },
};

/**
 * Retries spent — not retrying, and not waiting on you either. `flag` stays
 * `--fg-default` because the row's `--step-stopped-bg` surface already carries
 * the warning, and a hued flag would say it twice. The surface is the rail's;
 * see `Compositions/Workflow rail`.
 */
export const Stopped: Story = {
  args: { activity: "stopped", label: "Stopped" },
};

/**
 * A killed step takes no hue. Killing is a human decision rather than a system
 * failure and must not read as an error — that exclusion is the whole point of
 * the value. It borrows `power` from the Job badge that means the same thing
 * one level down; no roster names a glyph for it. Reported.
 */
export const Killed: Story = {
  args: { activity: "killed", label: "Killed" },
};

/**
 * The one value reporting an outcome rather than a position. Hued `x`, and the
 * row beneath carries `--step-failed-bg`: failed is an outcome and states it in
 * both channels.
 */
export const Failed: Story = {
  args: { activity: "failed", label: "Failed a check" },
};

/**
 * Every value in one column, which is how the set is read — the roster of
 * declarations in `packages/tokens/src/status.css` plus the two that
 * deliberately take none.
 */
export const EveryValue: StoryObj = {
  render: () => {
    const values: [StepActivity, string, number?][] = [
      ["not_started", "Not started", 3],
      ["running", "Running"],
      ["awaiting_human", "Waiting on you"],
      ["retrying", "Retrying"],
      ["advanced", "Advanced"],
      ["stopped", "Stopped"],
      ["killed", "Killed"],
      ["failed", "Failed a check"],
    ];
    return (
      <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
        {values.map(([activity, label, ordinal]) => (
          <div key={activity} style={{ display: "flex", alignItems: "center", gap: "var(--space-2)" }}>
            <StepActivityMark activity={activity} label={label} ordinal={ordinal} />
            <span style={{ fontFamily: "var(--font-mono)", fontSize: "var(--text-2xs)", color: "var(--fg-subtle)" }}>
              {activity}
            </span>
          </div>
        ))}
      </div>
    );
  },
};
