import type { Meta, StoryObj } from "@storybook/react-vite";

import { MachineSettings } from "./MachineSettings";

/** This machine's own settings — Helm's action authority first (#1089). */
const meta: Meta<typeof MachineSettings> = {
  title: "Compositions/Machine settings",
  component: MachineSettings,
};
export default meta;

type Story = StoryObj<typeof MachineSettings>;

export const HelmActionAuthority: Story = {
  name: "Helm action authority",
  args: {
    rows: [
      {
        label: "Helm action authority",
        value: "Acting",
        means:
          "Whether Helm may act on a Tier 1 Redirect on this machine, or only read one and suggest it. Fleet resolves this once, when it starts, and nothing here changes it while Fleet runs.",
      },
    ],
  },
};

/** Before `GET /health` has answered — the row still reads with no value line. */
export const HelmActionAuthorityUnread: Story = {
  name: "Helm action authority, not read yet",
  args: {
    rows: [
      {
        label: "Helm action authority",
        means:
          "Whether Helm may act on a Tier 1 Redirect on this machine, or only read one and suggest it. Fleet resolves this once, when it starts, and nothing here changes it while Fleet runs.",
      },
    ],
  },
};
