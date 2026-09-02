import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";
import { Switch } from "./Switch";

const meta: Meta<typeof Switch> = {
  title: "Primitives/Switch",
  component: Switch,
};
export default meta;

type Story = StoryObj<typeof Switch>;

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

/** On: `--accent` track, `--fg-default` thumb. */
export const On: Story = {
  render: () => (
    <Card>
      <Switch defaultChecked>Escalate on stall</Switch>
    </Card>
  ),
};

/** Off: `--bg-hover` track on a `--border-default` edge, `--fg-subtle` thumb. */
export const Off: Story = {
  render: () => (
    <Card>
      <Switch>Auto-approve small diffs</Switch>
    </Card>
  ),
};

/** A 2px `--accent` ring at 2px offset, on the track. */
export const Focused: Story = {
  render: () => (
    <Card>
      <Switch defaultChecked data-preview-focus="">
        Escalate on stall
      </Switch>
    </Card>
  ),
};

/** `--fg-subtle` with hover suppressed. Never opacity. */
export const Disabled: Story = {
  render: () => (
    <Card>
      <Switch defaultChecked disabled>
        Escalate on stall
      </Switch>
      <Switch disabled>Auto-approve small diffs</Switch>
    </Card>
  ),
  /**
   * **Disabled is set back, never opacity** — which means a disabled switch
   * looks like a quiet one, and nothing about how it is drawn says it is
   * inert. Pressing it is the only thing that does.
   *
   * Pressed by its label, because that is where a person presses a switch: the
   * whole row is the `label`, so a click anywhere in it forwards to the input.
   * A switch that took that press while disabled would flip a setting the
   * surface had already said was not available.
   */
  play: async ({ canvas, userEvent }) => {
    const on = canvas.getByRole("switch", { name: "Escalate on stall" });
    await expect(on).toBeChecked();

    await userEvent.click(canvas.getByText("Escalate on stall"));
    await expect(on).toBeChecked();
  },
};

/** The description says what happens on and what happens off. */
export const WithADescription: Story = {
  render: () => (
    <Card>
      <Switch
        defaultChecked
        description="A drone that stops reporting for 12 minutes reaches your phone. Off, it waits on the Alerts queue."
      >
        Escalate on stall
      </Switch>
      <Switch description="Armada reads the config repo each launch. Local edits made since the last push are kept.">
        Pull the Kit on startup
      </Switch>
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
        <Switch defaultChecked>Escalate on stall</Switch>
      </Card>
    </div>
  ),
};
