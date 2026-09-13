import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";

import { MANIFEST_ID, repository } from "../../../fixtures/build/base";
import { VERIFY_ENDED } from "../Manifest/Manifest";
import { endShownIn, pressable, scrollerOf } from "../Manifest/scrolled";
import { SCRATCH, SetupFrom } from "./Setup";

/**
 * Setup on the Manifest surface — Journey 3 — over a storefront nobody set up: the picker,
 * and each workspace's proposal opening over it and closing back to it, in any order.
 */
const meta = {
  title: "Screens/Setup",
  component: SetupFrom,
  parameters: { layout: "fullscreen" },
} satisfies Meta<typeof SetupFrom>;

export default meta;
type Story = StoryObj<typeof meta>;

/** Every workspace Scan found, ticked by evidence, with the gap the batch shows. */
export const ThePicker: Story = {
  name: "The picker",
  args: {},
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const list = await canvas.findByRole("region", { name: "Workspaces" });
    for (const dir of [".", "apps/web", "services/api", "docs"]) {
      await expect(within(list).getByRole("listitem", { name: dir })).toBeVisible();
    }
    await expect(within(within(list).getByRole("listitem", { name: "docs" })).getByRole("checkbox")).not.toBeChecked();
    await expect(within(within(list).getByRole("listitem", { name: "docs" })).getByText("no checks proposed")).toBeVisible();
    // This repository's own root already has a Manifest, so it is not offered as writable.
    await expect(within(within(list).getByRole("listitem", { name: "." })).getByText("already set up")).toBeVisible();
    // `lint` is declared by `apps/web` and not by `services/api`, its one ticked sibling.
    const grid = canvas.getByRole("region", { name: "Check names across the batch" });
    await expect(within(grid).getAllByText("missing")).toHaveLength(1);
  },
};

/**
 * A proposal opened, a line read for where it came from, a guess corrected, and the sheet
 * closed back to the picker, which then opens another. Not a wizard.
 */
export const CorrectAndClose: Story = {
  name: "Correct a line and close",
  args: {},
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const list = await canvas.findByRole("region", { name: "Workspaces" });
    await userEvent.click(within(list).getByRole("button", { name: "Open apps/web" }));
    const sheet = await canvas.findByRole("dialog", { name: "Proposal for apps/web" });
    const checks = within(sheet).getByRole("list", { name: "Checks" });
    const lint = within(checks).getByRole("listitem", { name: "lint" });
    await expect(within(lint).getByText("apps/web/package.json scripts.lint")).toBeVisible();
    await expect(within(lint).getByText("convention")).toBeVisible();

    await userEvent.click(within(lint).getByRole("button", { name: "Edit lint command" }));
    const field = within(lint).getByLabelText("lint command");
    await userEvent.clear(field);
    await userEvent.type(field, "pnpm eslint . --max-warnings 0{Enter}");
    await expect(await within(lint).findByText("edited during setup")).toBeVisible();
    await expect(within(lint).queryByText("convention")).toBeNull();

    // A guess moved in one press lands in the other registry.
    const dev = within(within(sheet).getByRole("list", { name: "Commands" })).getByRole("listitem", { name: "dev" });
    await userEvent.click(within(dev).getByRole("button", { name: "Move to Checks" }));
    await expect(await within(within(sheet).getByRole("list", { name: "Checks" })).findByRole("listitem", { name: "dev" })).toBeVisible();

    await userEvent.click(within(sheet).getByRole("button", { name: "Back to the workspaces" }));
    await expect(canvas.queryByRole("dialog")).toBeNull();
    await expect(within(within(list).getByRole("listitem", { name: "apps/web" })).getByText("ready to write")).toBeVisible();
    await userEvent.click(within(list).getByRole("button", { name: "Open services/api" }));
    const api = await canvas.findByRole("dialog", { name: "Proposal for services/api" });
    await expect(within(within(api).getByRole("list", { name: "Ports" })).getByText("compose.yaml services.db.ports")).toBeVisible();
  },
};

/**
 * A Check's prerequisite and a Command's destructive flag corrected from their values, and the
 * file Write puts down carrying both.
 */
export const CorrectRunsFirstAndDestructive: Story = {
  name: "Correct what runs first and what is destructive",
  args: { write: "took", onWritten: fn() },
  play: async ({ args, canvasElement }) => {
    const canvas = within(canvasElement);
    const list = await canvas.findByRole("region", { name: "Workspaces" });
    await userEvent.click(within(list).getByRole("button", { name: "Open services/api" }));
    const sheet = await canvas.findByRole("dialog", { name: "Proposal for services/api" });

    const test = within(within(sheet).getByRole("list", { name: "Checks" })).getByRole("listitem", { name: "test" });
    await userEvent.click(within(test).getByRole("button", { name: "Edit test runs first" }));
    const first = within(test).getByRole("dialog", { name: "test runs first" });
    await userEvent.click(within(first).getByRole("checkbox", { name: "migrate" }));
    await expect(await within(test).findByText("edited during setup")).toBeVisible();
    await expect(within(test).getByRole("button", { name: "Edit test runs first" })).toHaveTextContent("migrate →");
    await userEvent.click(within(test).getByRole("button", { name: "Edit test runs first" }));
    await expect(within(test).queryByRole("dialog")).toBeNull();

    const commands = within(sheet).getByRole("list", { name: "Commands" });
    const reset = within(commands).getByRole("listitem", { name: "reset" });
    await userEvent.click(within(reset).getByRole("button", { name: "Edit reset destructive" }));
    const flag = within(reset).getByRole("dialog", { name: "reset destructive" });
    await expect(within(flag).getByText(/this is your judgement/)).toBeVisible();
    await userEvent.click(within(flag).getByRole("switch", { name: /Destructive/ }));
    await expect(await within(reset).findByText("edited during setup")).toBeVisible();
    await expect(within(reset).getByRole("button", { name: "Edit reset destructive" })).toHaveTextContent(/^destructive$/);
    // Esc closes the popover and leaves the sheet it opened on.
    await expect(within(reset).getByRole("switch", { name: /Destructive/ })).toHaveFocus();
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(within(reset).queryByRole("dialog")).toBeNull());
    await expect(canvas.getByRole("dialog", { name: "Proposal for services/api" })).toBeVisible();

    // `test` runs `migrate` first now, so `migrate` is not offered as destructive.
    const migrate = within(commands).getByRole("listitem", { name: "migrate" });
    await userEvent.click(within(migrate).getByRole("button", { name: "Edit migrate destructive" }));
    await expect(within(migrate).getByRole("switch", { name: /Destructive/ })).toBeDisabled();
    await expect(within(migrate).getByText("test runs it first, so it cannot be destructive.")).toBeVisible();
    await userEvent.click(within(migrate).getByRole("button", { name: "Edit migrate destructive" }));
    await expect(within(sheet).queryByRole("dialog")).toBeNull();

    await userEvent.click(within(sheet).getByRole("button", { name: "Write services/api/armada.yml" }));
    await expect(await within(sheet).findByText(/^Wrote services\/api\/armada.yml at /)).toBeVisible();
    await expect(args.onWritten).toHaveBeenCalledTimes(1);
    const [file, text] = (args.onWritten as ReturnType<typeof fn>).mock.calls[0] as [string, string];
    await expect(file).toBe("services/api/armada.yml");
    await expect(text).toContain("  test:\n    run: pnpm test\n    requires:\n      - migrate\n");
    await expect(text).toContain("  reset:\n    run: pnpm reset\n    destructive: true\n");
  },
};

/** Both policies read in words on the sheet, each with what the selected value does. */
export const PolicyInWords: Story = {
  name: "Policy in words",
  args: {},
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const list = await canvas.findByRole("region", { name: "Workspaces" });
    await userEvent.click(within(list).getByRole("button", { name: "Open apps/web" }));
    const sheet = await canvas.findByRole("dialog", { name: "Proposal for apps/web" });
    const policy = within(sheet).getByRole("region", { name: "Policy" });
    await expect(within(policy).getByRole("radio", { name: "A person merges never" })).toBeChecked();
    await expect(within(policy).getByRole("radio", { name: "Fleet merges once the forge's checks pass checks-pass" })).not.toBeChecked();
    await expect(within(policy).getByText("A person merges every pull request here.")).toBeVisible();
    await userEvent.click(within(policy).getByRole("radio", { name: "Fleet merges whatever ran always" }));
    await expect(await within(policy).findByText("Fleet merges without waiting on the forge's checks.")).toBeVisible();
  },
};

/** Write, and the sheet that wrote a workspace's file offers Verify for that file, sending its workspace. */
export const Write: Story = {
  args: { write: "took", sheet: { state: "read", sheet: { setup: [], checks: [], commands: [] } }, onVerify: fn() },
  play: async ({ args, canvasElement }) => {
    const canvas = within(canvasElement);
    const list = await canvas.findByRole("region", { name: "Workspaces" });
    await userEvent.click(within(list).getByRole("button", { name: "Open apps/web" }));
    const sheet = await canvas.findByRole("dialog", { name: "Proposal for apps/web" });
    await userEvent.click(within(sheet).getByRole("button", { name: "Write apps/web/armada.yml" }));
    await expect(await within(sheet).findByText(/^Wrote apps\/web\/armada.yml at /)).toBeVisible();
    await expect(within(sheet).queryByRole("button", { name: /^Edit / })).toBeNull();
    const verify = within(sheet).getByRole("region", { name: "Verify" });
    await userEvent.click(within(verify).getByRole("button", { name: "Verify" }));
    await expect(args.onVerify).toHaveBeenCalledWith("apps/web");
  },
};

/** A workspace's Verify lands on the sheet that wrote its file, and not on another workspace's. */
export const WorkspaceVerifyLands: Story = {
  name: "A workspace's Verify lands on its sheet",
  args: {
    write: "took",
    sheet: { state: "read", sheet: { setup: [], checks: [], commands: [], verify: { ...VERIFY_ENDED, workspace: "apps/web" } } },
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const list = await canvas.findByRole("region", { name: "Workspaces" });
    await userEvent.click(within(list).getByRole("button", { name: "Open apps/web" }));
    const web = await canvas.findByRole("dialog", { name: "Proposal for apps/web" });
    await userEvent.click(within(web).getByRole("button", { name: "Write apps/web/armada.yml" }));
    const verify = await within(web).findByRole("region", { name: "Verify" });
    await expect(within(verify).getByText(/^Ran 4 of 4\./)).toBeVisible();

    await userEvent.click(within(web).getByRole("button", { name: "Back to the workspaces" }));
    await userEvent.click(within(list).getByRole("button", { name: "Open services/api" }));
    const api = await canvas.findByRole("dialog", { name: "Proposal for services/api" });
    await userEvent.click(within(api).getByRole("button", { name: "Write services/api/armada.yml" }));
    const theirs = await within(api).findByRole("region", { name: "Verify" });
    await expect(within(theirs).queryByText(/^Ran 4 of 4\./)).toBeNull();
    await expect(within(theirs).getByRole("button", { name: "Verify" })).toBeEnabled();
  },
};

/**
 * A root with an `armada.yml` already: its sheet offers no Write, and sends a person to the
 * Edit tab, which edits that file.
 */
export const AlreadySetUp: Story = {
  name: "A workspace already set up",
  args: {},
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const list = await canvas.findByRole("region", { name: "Workspaces" });
    await userEvent.click(within(list).getByRole("button", { name: "Open ." }));
    const sheet = await canvas.findByRole("dialog", { name: "Proposal for ." });
    await expect(within(sheet).getByText("armada.yml is already here")).toBeVisible();
    await expect(within(sheet).queryByRole("button", { name: "Write armada.yml" })).toBeNull();
    await userEvent.click(within(sheet).getByRole("button", { name: "Open the Edit tab" }));
    await expect(await canvas.findByRole("tab", { name: "Edit", selected: true })).toBeVisible();
  },
};

/** A root nobody set up lands on Verify after Write — the case a repository added by folder meets. */
export const WriteTheRoot: Story = {
  name: "Write the root, then Verify",
  args: { write: "took", rootSetUp: false, sheet: { state: "read", sheet: { setup: [], checks: [], commands: [] } } },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const list = await canvas.findByRole("region", { name: "Workspaces" });
    await userEvent.click(within(list).getByRole("button", { name: "Open ." }));
    const sheet = await canvas.findByRole("dialog", { name: "Proposal for ." });
    await userEvent.click(within(sheet).getByRole("button", { name: "Write armada.yml" }));
    const verify = await within(sheet).findByRole("region", { name: "Verify" });
    await expect(within(verify).getByRole("button", { name: "Verify" })).toBeEnabled();
  },
};

/** Write refused: the fault is under the row it names, and nothing was written. */
export const WriteRefused: Story = {
  name: "Write refused",
  args: { write: "refused" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(within(await canvas.findByRole("region", { name: "Workspaces" })).getByRole("button", { name: "Open apps/web" }));
    const sheet = await canvas.findByRole("dialog", { name: "Proposal for apps/web" });
    await userEvent.click(within(sheet).getByRole("button", { name: "Write apps/web/armada.yml" }));
    const lint = within(within(sheet).getByRole("list", { name: "Checks" })).getByRole("listitem", { name: "lint" });
    await expect(await within(lint).findByText("is empty.")).toBeVisible();
    await expect(within(sheet).getByText("Not written")).toBeVisible();
  },
};

/** A file was already at the path: nothing written over it, and the picker says so. */
export const AlreadyThere: Story = {
  name: "A file already there",
  args: { write: "appeared" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const list = await canvas.findByRole("region", { name: "Workspaces" });
    await userEvent.click(within(list).getByRole("button", { name: "Open apps/web" }));
    const sheet = await canvas.findByRole("dialog", { name: "Proposal for apps/web" });
    await userEvent.click(within(sheet).getByRole("button", { name: "Write apps/web/armada.yml" }));
    await expect(await within(sheet).findByText("Nothing was written over it. It reads:")).toBeVisible();
    await expect(within(sheet).queryByRole("button", { name: /^Write / })).toBeNull();
    await userEvent.click(within(sheet).getByRole("button", { name: "Back to the workspaces" }));
    await expect(within(within(list).getByRole("listitem", { name: "apps/web" })).getByText("already set up")).toBeVisible();
  },
};

/** The rail's picker over one Fleet's own repository and a folder added by path nobody set up. */
const TWO: Story["args"] = { repositories: [repository(), SCRATCH], sheet: { state: "read", sheet: { setup: [], checks: [], commands: [] } } };

/** Both listed, the one not set up grouped as such, and the Fleet's own picked. */
export const BothListed: Story = {
  name: "A set-up and a not-set-up repository",
  args: TWO,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const picker = canvas.getByRole("combobox", { name: "Project" });
    // A set-up repository reads as its Manifest id.
    await expect(within(picker).getByRole("option", { name: MANIFEST_ID })).toBeInTheDocument();
    const notSetUp = within(picker).getByRole("group", { name: "Not set up" });
    await expect(within(notSetUp).getByRole("option", { name: "scratch" })).toBeInTheDocument();
    await expect(within(notSetUp).queryByRole("option", { name: MANIFEST_ID })).toBeNull();
    await expect(picker).toHaveValue("/Users/user/armada");
    await expect(canvas.queryByRole("region", { name: "Workspaces" })).toBeNull();
  },
};

/** A workspace verifies on its own, in a repository whose root has no armada.yml. */
export const VerifyWorkspaceAlone: Story = {
  name: "Verify a workspace before the root is set up",
  args: { ...TWO, onVerify: fn() },
  play: async ({ args, canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.selectOptions(canvas.getByRole("combobox", { name: "Project" }), SCRATCH.root);
    const list = await canvas.findByRole("region", { name: "Workspaces" });
    await userEvent.click(within(list).getByRole("button", { name: "Open apps/web" }));
    const sheet = await canvas.findByRole("dialog", { name: "Proposal for apps/web" });
    await userEvent.click(within(sheet).getByRole("button", { name: "Write apps/web/armada.yml" }));
    const verify = await within(sheet).findByRole("region", { name: "Verify" });
    await expect(within(verify).queryByText(/at its root/)).toBeNull();
    await userEvent.click(within(verify).getByRole("button", { name: "Verify" }));
    await expect(args.onVerify).toHaveBeenCalledWith("apps/web");
  },
};

/** Pick the folder nobody set up: Setup alone opens for it, Write puts its root file down, and Verify is there. */
export const PickNotSetUp: Story = {
  name: "Pick one not set up, write it, verify",
  args: TWO,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const picker = canvas.getByRole("combobox", { name: "Project" });
    await userEvent.selectOptions(picker, SCRATCH.root);
    const list = await canvas.findByRole("region", { name: "Workspaces" });
    await expect(canvas.queryByRole("tab", { name: "Edit" })).toBeNull();
    await userEvent.click(within(list).getByRole("button", { name: "Open ." }));
    const sheet = await canvas.findByRole("dialog", { name: "Proposal for ." });
    await userEvent.click(within(sheet).getByRole("button", { name: "Write armada.yml" }));
    const verify = await within(sheet).findByRole("region", { name: "Verify" });
    await expect(within(verify).getByRole("button", { name: "Verify" })).toBeEnabled();
    await expect(within(picker).queryByRole("group", { name: "Not set up" })).toBeNull();
    await expect(within(picker).getByRole("option", { name: "scratch" })).toBeInTheDocument();
    await expect(picker).toHaveValue(SCRATCH.root);
    await expect(canvas.getByRole("tab", { name: "Edit" })).toBeVisible();
  },
};

/**
 * **A batch taller than the window.** The list and the grid scroll as one view under tabs that stay
 * put, and a proposal opened from the grid's last row comes up over them with a scroll of its own.
 */
export const ALongBatchScrolled: Story = {
  name: "A long batch, scrolled",
  args: { more: 16 },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const list = await canvas.findByRole("region", { name: "Workspaces" });
    const batch = canvas.getByRole("region", { name: "Check names across the batch" });
    const picker = scrollerOf(batch);
    await expect(picker).not.toBeNull();
    await expect(scrollerOf(list)).toBe(picker);
    await expect(picker!.scrollHeight).toBeGreaterThan(picker!.clientHeight);
    await expect(endShownIn(batch, picker!)).toBe(false);
    const tabsAt = canvas.getByRole("tablist").getBoundingClientRect().top;

    picker!.scrollTop = picker!.scrollHeight;
    await waitFor(() => expect(endShownIn(batch, picker!)).toBe(true));
    await expect(canvas.getByRole("tablist").getBoundingClientRect().top).toBe(tabsAt);
    const last = within(batch).getAllByRole("button", { name: "packages/pkg-16" });
    await expect(pressable(last[0]!)).toBe(true);

    await userEvent.click(last[0]!);
    const sheet = await canvas.findByRole("dialog", { name: "Proposal for packages/pkg-16" });
    const own = scrollerOf(within(sheet).getByRole("list", { name: "Checks" }));
    await expect(own).not.toBeNull();
    await expect(own).not.toBe(picker);
    await expect(sheet.contains(own) || own!.contains(sheet)).toBe(true);
    await userEvent.click(within(sheet).getByRole("button", { name: "Back to the workspaces" }));
    await expect(canvas.queryByRole("dialog")).toBeNull();
  },
};
