import type { Meta, StoryObj } from "@storybook/react-vite";
import { Checkbox } from "./Checkbox";

const meta: Meta<typeof Checkbox> = {
  title: "Primitives/Checkbox",
  component: Checkbox,
};
export default meta;

type Story = StoryObj<typeof Checkbox>;

function Card({ children }: { children: React.ReactNode }) {
  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "var(--space-2)",
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

/** Unchecked: the input's own `--bg-sunken` well and `--border-default` edge. */
export const Unchecked: Story = {
  render: () => (
    <Card>
      <Checkbox>Land as a convoy</Checkbox>
    </Card>
  ),
};

/** Checked: `--accent` fill, `check` in `--fg-inverse`. */
export const Checked: Story = {
  render: () => (
    <Card>
      <Checkbox defaultChecked>Run Doctor before dispatch</Checkbox>
    </Card>
  ),
};

/** A 2px `--accent` ring at 2px offset, on the box rather than the label. */
export const Focused: Story = {
  render: () => (
    <Card>
      <Checkbox defaultChecked data-preview-focus="">
        Run Doctor before dispatch
      </Checkbox>
    </Card>
  ),
};

/** `--fg-subtle` with hover suppressed. Never opacity. */
export const Disabled: Story = {
  render: () => (
    <Card>
      <Checkbox disabled>Land as a convoy</Checkbox>
      <Checkbox disabled defaultChecked>
        Run Doctor before dispatch
      </Checkbox>
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
        <Checkbox defaultChecked>Run Doctor before dispatch</Checkbox>
      </Card>
    </div>
  ),
};
