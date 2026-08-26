import type { Meta, StoryObj } from "@storybook/react-vite";
import { Input } from "./Input";

const meta: Meta<typeof Input> = {
  title: "Primitives/Input",
  component: Input,
};
export default meta;

type Story = StoryObj<typeof Input>;

/** A card — the ground a field's sunken well is measured against. */
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

/** At rest: `--bg-sunken` well, `--border-default` edge, `--fg-default` text. */
export const Default: Story = {
  args: { label: "Job title", defaultValue: "Refresh the auth token flow" },
  render: (args) => (
    <Card>
      <Input {...args} />
    </Card>
  ),
};

/** Placeholder is `--fg-subtle`, the same step as a timestamp. */
export const Placeholder: Story = {
  args: { label: "Job title", placeholder: "Refresh the auth token flow" },
  render: (args) => (
    <Card>
      <Input {...args} />
    </Card>
  ),
};

/** A path is machine-derived, so it is mono at one step smaller than the label. */
export const Mono: Story = {
  args: { label: "Project location", defaultValue: "~/code/armada", mono: true },
  render: (args) => (
    <Card>
      <Input {...args} />
    </Card>
  ),
};

/** A 2px `--accent` ring at 2px offset, no glow. */
export const Focused: Story = {
  render: () => (
    <Card>
      <Input label="Job title" defaultValue="Refresh the auth token flow" data-preview-focus="" />
    </Card>
  ),
};

/** `--status-completed-failed` border, and the message below it in `--text-xs`. */
export const Invalid: Story = {
  render: () => (
    <Card>
      <Input
        label="Branch"
        defaultValue="feat/auth"
        mono
        invalid
        message="Branch already exists in the workspace."
      />
    </Card>
  ),
};

/** `--fg-subtle` text with hover suppressed. Never opacity. */
export const Disabled: Story = {
  render: () => (
    <Card>
      <Input label="Job title" defaultValue="Refresh the auth token flow" disabled />
    </Card>
  ),
};

/**
 * Dark is primary and a light story is the secondary case. No light theme
 * exists in `packages/tokens` — one `:root` block, nothing keyed to a theme —
 * so this renders dark. Written so the gap is visible rather than absent.
 */
export const Light: Story = {
  render: () => (
    <div data-theme="light">
      <Card>
        <Input label="Job title" defaultValue="Refresh the auth token flow" />
      </Card>
    </div>
  ),
};
