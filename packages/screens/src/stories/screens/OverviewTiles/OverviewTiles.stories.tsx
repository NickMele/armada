import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
import { ARMADA, CONNECTED, drift, FLEET, health, NOW, OverviewTilesFrom, STOREFRONT } from "./OverviewTiles";

/**
 * Overview's five tiles, drawn by the app's own `OverviewTiles` from readings in the shapes Fleet
 * sends. One story per state #919 names: Fleet in each of its three, Doctor warning and failing,
 * and Queued and drift on All and on one pick.
 */
const meta = {
  title: "Screens/Overview tiles",
  component: OverviewTilesFrom,
  parameters: { layout: "padded" },
} satisfies Meta<typeof OverviewTilesFrom>;

export default meta;
type Story = StoryObj<typeof meta>;

/** All repositories, where Bridge opens: everything reads, and one repository is behind. */
export const OnAll: Story = {
  name: "On All repositories",
  args: { onOpenFleetSettings: fn(), onOpenQueued: fn(), onOpenManifest: fn() },
  play: async ({ args, canvas, userEvent }) => {
    // Fleet and Doctor open nothing until Doctor ships, so neither is a control.
    await expect(canvas.getByRole("group", { name: "Fleet" })).toBeVisible();
    await expect(canvas.getByRole("group", { name: "Doctor" })).toBeVisible();
    await expect(canvas.getAllByRole("button")).toHaveLength(3);
    await userEvent.click(canvas.getByRole("button", { name: /Open Queued on the Board/ }));
    await expect(args.onOpenQueued).toHaveBeenCalledOnce();
    await userEvent.click(canvas.getByRole("button", { name: /Open Fleet settings/ }));
    await expect(args.onOpenFleetSettings).toHaveBeenCalledOnce();
    await userEvent.click(canvas.getByRole("button", { name: /Open Manifest/ }));
    await expect(args.onOpenManifest).toHaveBeenCalledOnce();
  },
};

/** One repository picked: Queued and drift narrow to it, and the machine's three do not. */
export const OnePicked: Story = {
  name: "One repository picked",
  args: { picked: STOREFRONT.root },
};

/** A repository with no Manifest is named apart from those behind. */
export const OnAllWithOneNotSetUp: Story = {
  name: "On All, one not set up",
  args: {
    repositories: [ARMADA, STOREFRONT, { root: "/Users/user/code/scratch", records_root: "/records/scratch" }],
    drifts: {
      state: "held",
      repositories: [
        { root: ARMADA.root, drift: drift(8, 1) },
        { root: STOREFRONT.root, drift: drift(6, 2) },
        { root: "/Users/user/code/scratch", drift: { state: "failed", outcome: { ok: false, why: "not_set_up" } } },
      ],
    },
  },
};

/** A live pid that does not answer: amber, and the readings behind it could not be taken. */
export const FleetUnreachable: Story = {
  name: "Fleet unreachable",
  args: {
    connection: { state: "unreachable", fleet: FLEET, detail: "", sinceMs: NOW - 20_000 },
    readAt: NOW - 20_000,
    healthRead: { state: "failed", outcome: { ok: false, why: "not_connected" } },
    capacity: null,
  },
};

/** No runtime file: red, and nothing else can be asked. */
export const FleetNotRunning: Story = {
  name: "Fleet not running",
  args: {
    connection: { state: "not_running", absence: { why: "no_runtime_file", path: "~/.armada/fleet.json" } },
    healthRead: { state: "failed", outcome: { ok: false, why: "not_connected" } },
    capacity: null,
    drifts: {
      state: "held",
      repositories: [
        { root: ARMADA.root, drift: { state: "failed", outcome: { ok: false, why: "not_connected" } } },
        { root: STOREFRONT.root, drift: { state: "failed", outcome: { ok: false, why: "not_connected" } } },
      ],
    },
  },
};

export const DoctorWarn: Story = {
  name: "Doctor warns",
  args: { healthRead: health({ Manifest: ["warn", "1 of 2 has drifted"] }) },
};

export const DoctorFail: Story = {
  name: "Doctor fails",
  args: { healthRead: health({ SQLite: ["fail", "armada.db is locked"], Manifest: ["warn", "1 of 2 has drifted"] }) },
};

/** Every Drone busy and the next held back on memory, with Jobs waiting on it. */
export const DronesHeld: Story = {
  name: "Drones held",
  args: { capacity: { bound: 2, occupied: 2, held_by: "memory" } },
};

/** Opened a moment ago: the connection is known and nothing it serves has answered. */
export const NotReadYet: Story = {
  name: "Not read yet",
  args: { connection: CONNECTED, healthRead: { state: "reading" }, capacity: null, drifts: { state: "none" } },
};
