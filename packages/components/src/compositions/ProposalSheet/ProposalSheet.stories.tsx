import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { ProposalSheet, type ProposalSheetProps } from "./ProposalSheet";

/**
 * A storefront's `apps/web` proposal: two scripts Scan guessed were Checks, one it guessed was
 * a Command, no port a file declares, and both policies at default.
 */
const meta: Meta<typeof ProposalSheet> = {
  title: "Compositions/Proposal sheet",
  component: ProposalSheet,
  parameters: { layout: "fullscreen" },
};
export default meta;

type Story = StoryObj<typeof ProposalSheet>;

const CAPS = "Jobs here stop at $5.00 or 200 turns. Change these on the Manifest page, where past Jobs' costs are.";

const BASE: ProposalSheetProps = {
  open: true,
  dir: "apps/web",
  file: "apps/web/armada.yml",
  id: { value: "web", cited: { source: "convention", file: "apps/web/package.json", at: "name" } },
  ports: [],
  checks: [
    { name: "test", run: "pnpm vitest run", cited: { source: "convention", file: "apps/web/package.json", at: "scripts.test" } },
    { name: "lint", run: "pnpm eslint .", cited: { source: "convention", file: "apps/web/package.json", at: "scripts.lint" } },
  ],
  commands: [
    { name: "dev", run: "pnpm vite", cited: { source: "convention", file: "apps/web/package.json", at: "scripts.dev" } },
    { name: "reset", run: "pnpm db:reset", destructive: true, cited: { source: "convention", file: "apps/web/package.json", at: "scripts.db:reset" } },
  ],
  policy: [
    {
      key: "auto_merge",
      label: "Auto merge",
      value: "never",
      options: [
        { value: "never", reads: "A person merges", says: "A person merges every pull request here." },
        { value: "checks-pass", reads: "Fleet merges once the forge's checks pass", says: "Fleet merges once every check the forge runs has passed." },
      ],
      cited: { source: "default" },
    },
  ],
  caps: CAPS,
  onEditId: fn(),
  onEditRun: fn(),
  onRequires: fn(),
  onDestructive: fn(),
  onMove: fn(),
  onRemove: fn(),
  onAdd: fn(),
  onAddPort: fn(),
  onPolicy: fn(),
  onWrite: fn(),
  onClose: fn(),
};

/** Every line cites its source, a value is edited where it sits, and a guess moves in one press. */
export const Proposed: Story = {
  args: BASE,
  play: async ({ args, canvasElement, userEvent }) => {
    const sheet = within(canvasElement);
    const checks = sheet.getByRole("list", { name: "Checks" });
    const lint = within(checks).getByRole("listitem", { name: "lint" });
    await expect(within(lint).getByText("convention")).toBeVisible();

    await userEvent.click(within(lint).getByRole("button", { name: "Edit lint command" }));
    const field = within(lint).getByLabelText("lint command");
    await userEvent.clear(field);
    await userEvent.type(field, "pnpm eslint . --max-warnings 0{Enter}");
    await expect(args.onEditRun).toHaveBeenCalledWith("checks", "lint", "pnpm eslint . --max-warnings 0");

    await userEvent.click(within(lint).getByRole("button", { name: "Move to Commands" }));
    await expect(args.onMove).toHaveBeenCalledWith("checks", "lint");
    await expect(sheet.getByText(/No file here declares one/)).toBeVisible();
    await expect(sheet.queryByRole("button", { name: /Helm/ })).toBeNull();
  },
};

/** A Check's prerequisites and a Command's flag, each opened from its value; the edit goes out whole. */
export const RunsFirstAndDestructive: Story = {
  name: "Runs first, and destructive",
  args: BASE,
  play: async ({ args, canvasElement, userEvent }) => {
    const sheet = within(canvasElement);
    const test = within(sheet.getByRole("list", { name: "Checks" })).getByRole("listitem", { name: "test" });
    await userEvent.click(within(test).getByRole("button", { name: "Edit test runs first" }));
    const first = within(test).getByRole("dialog", { name: "test runs first" });
    // A destructive Command is shown and not offered: the file would not load.
    await expect(within(first).getByRole("checkbox", { name: "reset" })).toBeDisabled();
    await userEvent.click(within(first).getByRole("checkbox", { name: "dev" }));
    await expect(args.onRequires).toHaveBeenCalledWith("test", ["dev"]);
    await userEvent.click(within(test).getByRole("button", { name: "Edit test runs first" }));

    const reset = within(sheet.getByRole("list", { name: "Commands" })).getByRole("listitem", { name: "reset" });
    await userEvent.click(within(reset).getByRole("button", { name: "Edit reset destructive" }));
    await userEvent.click(within(reset).getByRole("switch", { name: /Destructive/ }));
    await expect(args.onDestructive).toHaveBeenCalledWith("reset", false);
    await userEvent.click(within(reset).getByRole("button", { name: "Edit reset destructive" }));
    await expect(sheet.queryByRole("dialog", { name: /runs first|destructive/ })).toBeNull();

    await expect(within(sheet.getByRole("region", { name: "Policy" })).getByRole("radio", { name: "A person merges never" })).toBeChecked();
  },
};

/** A line a person corrected reads so in sans, where the file sat. */
export const Edited: Story = {
  args: {
    ...BASE,
    checks: [
      { ...BASE.checks[0]!, run: "pnpm vitest run --coverage", cited: { source: "edited_during_setup" } },
      BASE.checks[1]!,
    ],
  },
  play: async ({ canvasElement }) => {
    const test = within(within(canvasElement).getByRole("list", { name: "Checks" })).getByRole("listitem", { name: "test" });
    await expect(within(test).getByText("edited during setup")).toBeVisible();
    await expect(within(test).queryByText(/package\.json/)).toBeNull();
  },
};

/** Write refused: every fault listed at the foot, and each drawn under the row it names. */
export const Refused: Story = {
  args: {
    ...BASE,
    checks: [{ ...BASE.checks[0]!, requires: ["seed"] }],
    refused: {
      saying: "apps/web/armada.yml would not load.",
      faults: [{ key: "checks.test.requires", fault: "names `seed`, which no Command declares." }],
    },
  },
  play: async ({ canvasElement }) => {
    const sheet = within(canvasElement);
    await expect(sheet.getByText("Not written")).toBeVisible();
    const test = within(sheet.getByRole("list", { name: "Checks" })).getByRole("listitem", { name: "test" });
    await expect(within(test).getByText("names `seed`, which no Command declares.")).toBeVisible();
  },
};

/** A file was already at the path. Nothing was written over it, and what is there is shown. */
export const AlreadyThere: Story = {
  args: { ...BASE, appeared: { onDisk: "version: 1\nid: web\n" } },
  play: async ({ canvasElement }) => {
    const sheet = within(canvasElement);
    await expect(sheet.getByText("Nothing was written over it. It reads:")).toBeVisible();
    await expect(sheet.getByText(/id: web/)).toBeVisible();
  },
};

/** Already set up: no Write and no edit, and the Edit tab offered where the file is reachable. */
export const AlreadySetUp: Story = {
  name: "Already set up",
  args: { ...BASE, setUp: true, onEditManifest: fn() },
  play: async ({ args, canvasElement, userEvent }) => {
    const sheet = within(canvasElement);
    await expect(sheet.queryByRole("button", { name: /^Write / })).toBeNull();
    await expect(sheet.queryByRole("button", { name: /^Edit / })).toBeNull();
    await userEvent.click(sheet.getByRole("button", { name: "Open the Edit tab" }));
    await expect(args.onEditManifest).toHaveBeenCalledTimes(1);
  },
};

/** Written: the receipt beside the press, Verify on the sheet that wrote the file, and no edit offered after. */
export const Written: Story = {
  args: {
    ...BASE,
    written: "Wrote apps/web/armada.yml at 14:20. Nothing was staged or committed.",
    verify: <section aria-label="Verify">Verify</section>,
  },
  play: async ({ canvasElement }) => {
    const sheet = within(canvasElement);
    await expect(sheet.getByRole("region", { name: "Verify" })).toBeVisible();
    await expect(sheet.getByRole("button", { name: "Written" })).toBeDisabled();
    await expect(sheet.getByText(/^Wrote apps\/web\/armada.yml/)).toBeVisible();
    await expect(sheet.queryByRole("button", { name: /^Edit / })).toBeNull();
    await expect(sheet.queryByRole("button", { name: "Remove" })).toBeNull();
  },
};
