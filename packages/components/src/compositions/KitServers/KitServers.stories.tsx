import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { KitServers, type KitServerRowProps } from "./KitServers";

/**
 * The MCP servers in a person's kit, and what a drone dispatched against this
 * repository is handed — the Manifest surface's Servers view (#1275). A plain
 * section rather than a layer, so no decorator stands in for a window.
 */
const meta: Meta<typeof KitServers> = {
  title: "Compositions/Kit servers",
  component: KitServers,
  args: { here: "armada" },
};
export default meta;

type Story = StoryObj<typeof KitServers>;

const OFF: KitServerRowProps = {
  name: "tracker",
  address: "npx -y @scope/server-tracker",
  kind: "stdio",
  reachesByDefault: false,
  resolves: false,
};

const EVERYWHERE: KitServerRowProps = {
  name: "nexus",
  address: "https://nexus.example.com/mcp",
  kind: "http",
  reachesByDefault: true,
  resolves: true,
};

/** Nothing read yet. Not an empty kit — nobody has asked one. */
export const Reading: Story = {
  name: "Reading",
  args: { servers: undefined },
};

/** A kit with nothing in it. A drone here gets Armada's own tool and no other. */
export const Empty: Story = {
  name: "Empty",
  args: { servers: [], onAdd: fn() },
};

/**
 * A server in kit, and nobody has allowed it anywhere. **The row says so** —
 * adding a server is not turning one on, which is the confinement a drone's
 * `--strict-mcp-config` keeps.
 */
export const InKitAndUnreached: Story = {
  name: "In Kit and unreached",
  args: {
    servers: [OFF],
    onAdd: fn(),
    onForget: fn(),
    onKitReach: fn(),
    onHereReach: fn(),
  },
  /**
   * **A row's two controls answer for two different tiers**, which a rendering
   * cannot show: both read "off" on this row, and only one of them is about
   * this repository. The `play` is what proves a press reaches the right one.
   */
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByText("Does not")).toBeInTheDocument();

    await userEvent.selectOptions(canvas.getByRole("combobox", { name: "Here: tracker" }), "extended");
    await expect(args.onHereReach).toHaveBeenCalledWith("tracker", "extended");
    await expect(args.onKitReach).not.toHaveBeenCalled();

    await userEvent.selectOptions(canvas.getByRole("combobox", { name: "In Kit: tracker" }), "yes");
    await expect(args.onKitReach).toHaveBeenCalledWith("tracker", true);
  },
};

/** The two tiers disagreeing, each way round, with Fleet's answer beside them. */
export const BothTiers: Story = {
  name: "Both tiers",
  args: {
    servers: [
      { ...OFF, here: "extended", resolves: true },
      { ...EVERYWHERE, here: "restricted", resolves: false },
    ],
    onForget: fn(),
    onKitReach: fn(),
    onHereReach: fn(),
  },
};

/**
 * Following kit again is a third state and not the absence of a second, so a
 * row that has been withheld here can be put back.
 */
export const BackToFollowingKit: Story = {
  name: "Back to following Kit",
  args: {
    servers: [{ ...EVERYWHERE, here: "restricted", resolves: false }],
    onHereReach: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.selectOptions(canvas.getByRole("combobox", { name: "Here: nexus" }), "kit");
    await expect(args.onHereReach).toHaveBeenCalledWith("nexus", null);
  },
};

/**
 * Adding one. **The arguments stay on the caller's side of the line** — this
 * hands over what was typed, and the split into a program and its arguments
 * happens once, beside the wire.
 */
export const Adding: Story = {
  name: "Adding",
  args: { servers: [], onAdd: fn() },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.type(canvas.getByLabelText("Name"), "tracker");
    await userEvent.type(canvas.getByLabelText("Program"), "npx -y @scope/server");
    await userEvent.click(canvas.getByRole("button", { name: "Add" }));
    await expect(args.onAdd).toHaveBeenCalledWith({
      name: "tracker",
      kind: "stdio",
      address: "npx -y @scope/server",
    });
  },
};

/** Fleet refused the name or the address, read exactly as it answered. */
export const Refused: Story = {
  name: "Refused",
  args: {
    servers: [OFF],
    onAdd: fn(),
    refused: "`armada` is Armada's own server, and a second one would leave a Drone's brief naming two",
  },
};
