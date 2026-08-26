import type { Meta, StoryObj } from "@storybook/react-vite";
import { Select } from "./Select";

const meta: Meta<typeof Select> = {
  title: "Primitives/Select",
  component: Select,
};
export default meta;

type Story = StoryObj<typeof Select>;

function Card({ children }: { children: React.ReactNode }) {
  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "var(--space-4)",
        width: "var(--sidebar-max)",
        padding: "var(--pad-card)",
        borderRadius: "var(--radius-md)",
        background: "var(--bg-raised)",
      }}
    >
      {children}
    </div>
  );
}

const ceilings = (
  <>
    <option>4 drones</option>
    <option>8 drones</option>
  </>
);

/** At rest, the input's box exactly: `--bg-sunken`, `--border-default`, 36px. */
export const Default: Story = {
  render: () => (
    <Card>
      <Select label="Concurrency ceiling">{ceilings}</Select>
    </Card>
  ),
};

/** A 2px `--accent` ring at 2px offset, no glow. */
export const Focused: Story = {
  render: () => (
    <Card>
      <Select label="Concurrency ceiling" data-preview-focus="">
        {ceilings}
      </Select>
    </Card>
  ),
};

/** `--status-completed-failed` border, message below in `--text-xs`. */
export const Invalid: Story = {
  render: () => (
    <Card>
      <Select
        label="Concurrency ceiling"
        invalid
        message="8 drones exceeds the machine's headroom."
        defaultValue="8 drones"
      >
        {ceilings}
      </Select>
    </Card>
  ),
};

/** `--fg-subtle` text with hover suppressed. Never opacity. */
export const Disabled: Story = {
  render: () => (
    <Card>
      <Select label="Concurrency ceiling" disabled>
        {ceilings}
      </Select>
    </Card>
  ),
};

/**
 * Dark is primary and a light story is the secondary case. No light theme
 * exists in `packages/tokens`, so this renders dark — written so the gap is
 * visible rather than absent.
 */
export const Light: Story = {
  render: () => (
    <div data-theme="light">
      <Card>
        <Select label="Concurrency ceiling">{ceilings}</Select>
      </Card>
    </div>
  ),
};
