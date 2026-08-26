import type { Meta, StoryObj } from "@storybook/react-vite";
import { Radio, RadioGroup } from "./Radio";

const meta: Meta<typeof Radio> = {
  title: "Primitives/Radio",
  component: Radio,
};
export default meta;

type Story = StoryObj<typeof Radio>;

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

/** A set at rest: the chosen ring and dot take `--accent`, the rest the
 *  input's own `--border-default` edge over a `--bg-sunken` well. */
export const Default: Story = {
  render: () => (
    <Card>
      <RadioGroup label="Kit source">
        <Radio name="kit" defaultChecked>
          Import an existing Kit
        </Radio>
        <Radio name="kit">Start fresh</Radio>
      </RadioGroup>
    </Card>
  ),
};

/** A 2px `--accent` ring at 2px offset, on the ring rather than the label. */
export const Focused: Story = {
  render: () => (
    <Card>
      <RadioGroup label="Kit source">
        <Radio name="kit-focus" defaultChecked data-preview-focus="">
          Import an existing Kit
        </Radio>
        <Radio name="kit-focus">Start fresh</Radio>
      </RadioGroup>
    </Card>
  ),
};

/** `--fg-subtle` with hover suppressed. Never opacity. */
export const Disabled: Story = {
  render: () => (
    <Card>
      <RadioGroup label="Kit source">
        <Radio name="kit-disabled" defaultChecked disabled>
          Import an existing Kit
        </Radio>
        <Radio name="kit-disabled" disabled>
          Start fresh
        </Radio>
      </RadioGroup>
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
        <RadioGroup label="Kit source">
          <Radio name="kit-light" defaultChecked>
            Import an existing Kit
          </Radio>
          <Radio name="kit-light">Start fresh</Radio>
        </RadioGroup>
      </Card>
    </div>
  ),
};
