import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";

import { MANIFEST_ID, repository } from "../../../fixtures/build/base";
import { CHOSEN, LocateFrom, TwoWindowsFrom } from "./Locate";

/**
 * Locate — Journey 3's *Getting in* — from the picker's own **Add a repository**: a folder added,
 * or a clone from a URL, landing on Setup for what Fleet now serves. Nothing native opens: the
 * folder dialog answers `/Users/user/scratch`.
 */
const meta = {
  title: "Screens/Locate",
  component: LocateFrom,
  parameters: { layout: "fullscreen" },
  args: { repositories: [repository()], onAdded: fn(), onCloned: fn() },
} satisfies Meta<typeof LocateFrom>;

export default meta;
type Story = StoryObj<typeof meta>;

const URL = "https://forge.invalid/owner/storefront.git";

/** The default fixture's picker reads the fixed repository's own Manifest id. */
const DEFAULT_LABEL = MANIFEST_ID;

/** Add a repository sits inside the picker's own menu, below a separator — opening it is opening the picker first. */
async function opened(canvasElement: HTMLElement, pickerLabel = DEFAULT_LABEL) {
  const canvas = within(canvasElement);
  await userEvent.click(canvas.getByRole("button", { name: pickerLabel }));
  await userEvent.click(canvas.getByRole("menuitem", { name: "Add a repository" }));
  return { canvas, dialog: canvas.getByRole("dialog", { name: "Add a repository" }) };
}

async function cloneFrom(canvasElement: HTMLElement, pickerLabel = DEFAULT_LABEL) {
  const { canvas, dialog } = await opened(canvasElement, pickerLabel);
  await userEvent.click(within(dialog).getByRole("button", { name: "Clone from a URL" }));
  await userEvent.type(within(dialog).getByLabelText("Repository URL"), URL);
  await userEvent.type(within(dialog).getByLabelText("Clone into"), "/Users/user/code");
  // Named before anything is pressed, as Fleet will name it.
  const lands = within(dialog).getByRole("group", { name: "Project location" });
  await expect(within(lands).getByText("/Users/user/code/storefront")).toBeVisible();
  await userEvent.click(within(dialog).getByRole("button", { name: /^Clone repository/ }));
  return { canvas, dialog };
}

/** A folder chosen and added: the dialog closes, the picker holds it, and Setup is open for it. */
export const AFolderAdded: Story = {
  name: "A folder added",
  play: async ({ args, canvasElement }) => {
    const { canvas, dialog } = await opened(canvasElement);
    await userEvent.click(within(dialog).getByRole("button", { name: "Choose a folder" }));
    await expect(within(dialog).getByLabelText("Project location")).toHaveValue(CHOSEN);
    await userEvent.click(within(dialog).getByRole("button", { name: /^Add repository/ }));
    await expect(args.onAdded).toHaveBeenCalledWith(CHOSEN);
    await waitFor(() => expect(canvas.queryByRole("dialog", { name: "Add a repository" })).toBeNull());
    // The added folder is nobody's Manifest yet, so it reads by its own name — the picker's label.
    const picker = canvas.getByRole("button", { name: "scratch" });
    await userEvent.click(picker);
    // Scoped to the open menu: the left column's Stats panel reads the same
    // two words for a repository's own setup state, #1088.
    const menu = canvas.getByRole("menu");
    await waitFor(() => expect(within(menu).getByText("Not set up")).toBeVisible());
    await expect(within(menu).getByRole("menuitem", { name: "scratch" })).toBeInTheDocument();
    await userEvent.keyboard("{Escape}");
    await expect(await canvas.findByRole("region", { name: "Workspaces" })).toBeVisible();
  },
};

/** A fresh install: Fleet serves nothing, so the dialog opens by itself over a picker saying so, and nothing reads as a fault. */
export const NothingServed: Story = {
  name: "Nothing served",
  args: { repositories: [] },
  play: async ({ args, canvasElement }) => {
    const canvas = within(canvasElement);
    const dialog = await canvas.findByRole("dialog", { name: "Add a repository" });
    await expect(canvas.getByRole("button", { name: "Nothing set up yet" })).toBeInTheDocument();
    await expect(canvas.getByText("Nothing is set up yet")).toBeInTheDocument();
    await expect(canvas.queryByText(/could not be read/)).toBeNull();
    await userEvent.type(within(dialog).getByLabelText("Project location"), CHOSEN);
    await userEvent.click(within(dialog).getByRole("button", { name: /^Add repository/ }));
    await expect(args.onAdded).toHaveBeenCalledWith(CHOSEN);
    // The first repository added lands on Setup.
    await expect(await canvas.findByRole("region", { name: "Workspaces" })).toBeVisible();
    await expect(canvas.getByRole("button", { name: "scratch" })).toBeInTheDocument();
  },
};

/** A clone that finished after its dialog closed says so, and moves nothing until its Setup is asked for. */
export const ACloneFinishedLate: Story = {
  name: "A clone that finished after its dialog closed",
  args: { clone: "late" },
  play: async ({ canvasElement }) => {
    const { canvas, dialog } = await cloneFrom(canvasElement);
    await userEvent.click(within(dialog).getByRole("button", { name: /^Cancel/ }));
    await waitFor(() => expect(canvas.queryByRole("dialog", { name: "Add a repository" })).toBeNull());
    await expect(await canvas.findByText("storefront is ready to set up", {}, { timeout: 3000 })).toBeVisible();
    await expect(canvas.getByRole("button", { name: DEFAULT_LABEL })).toBeInTheDocument();
    await expect(canvas.queryByRole("region", { name: "Workspaces" })).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "Open Setup" }));
    await expect(await canvas.findByRole("region", { name: "Workspaces" })).toBeVisible();
    await expect(canvas.getByRole("button", { name: "storefront" })).toBeInTheDocument();
    await expect(canvas.queryByText("storefront is ready to set up")).toBeNull();
  },
};

/** The person moved to another window: the clone is announced there too, and neither window's pick moves. #926. */
export const ACloneHeardInAnotherWindow: Story = {
  name: "A late clone, heard in another window",
  args: { clone: "late" },
  render: (args) => <TwoWindowsFrom {...args} />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const asked = canvas.getByRole("region", { name: "The window that asked" });
    const other = canvas.getByRole("region", { name: "Another window" });
    const { dialog } = await cloneFrom(asked);
    await userEvent.click(within(dialog).getByRole("button", { name: /^Cancel/ }));
    await expect(await within(other).findByText("storefront is ready to set up", {}, { timeout: 3000 })).toBeVisible();
    await expect(within(asked).getByText("storefront is ready to set up")).toBeVisible();
    for (const window of [asked, other]) {
      await expect(within(window).getByRole("button", { name: DEFAULT_LABEL })).toBeInTheDocument();
      await expect(within(window).queryByRole("region", { name: "Workspaces" })).toBeNull();
    }
    await userEvent.click(within(other).getByRole("button", { name: "Open Setup" }));
    await expect(await within(other).findByRole("region", { name: "Workspaces" })).toBeVisible();
    await expect(within(other).getByRole("button", { name: "storefront" })).toBeInTheDocument();
    await userEvent.click(within(asked).getByRole("button", { name: "Dismiss" }));
    await expect(canvas.queryByText("storefront is ready to set up")).toBeNull();
  },
};

/** A parent under a symlink previews the folder Fleet clones into, as main resolves it. */
export const AParentUnderASymlink: Story = {
  name: "A parent under a symlink",
  play: async ({ canvasElement }) => {
    const { dialog } = await opened(canvasElement);
    await userEvent.click(within(dialog).getByRole("button", { name: "Clone from a URL" }));
    await userEvent.type(within(dialog).getByLabelText("Repository URL"), URL);
    await userEvent.type(within(dialog).getByLabelText("Clone into"), "/tmp");
    const lands = within(dialog).getByRole("group", { name: "Project location" });
    await expect(await within(lands).findByText("/private/tmp/storefront")).toBeVisible();
    await userEvent.click(within(dialog).getByRole("button", { name: /^Cancel/ }));
  },
};

/** A clone underway says so, and a second press — the button or Enter — sends nothing. */
export const ACloneUnderway: Story = {
  name: "A clone underway",
  args: { clone: "underway" },
  play: async ({ args, canvasElement }) => {
    const { dialog } = await cloneFrom(canvasElement);
    await expect(await within(dialog).findByText(/Git is cloning into/)).toBeVisible();
    await expect(within(dialog).getByRole("button", { name: /^Clone repository/ })).toBeDisabled();
    await userEvent.keyboard("{Enter}");
    await expect(args.onCloned).toHaveBeenCalledTimes(1);
    await expect(args.onCloned).toHaveBeenCalledWith(URL, "/Users/user/code");
  },
};

/** Git's refusal reads in the dialog in full, under what did not happen, and nothing moves behind it. */
export const ACloneRefused: Story = {
  name: "A clone refused",
  args: { clone: "refused" },
  play: async ({ canvasElement }) => {
    const { canvas, dialog } = await cloneFrom(canvasElement);
    await expect(await within(dialog).findByText("Not cloned")).toBeVisible();
    await expect(within(dialog).getByText(`git refused the clone: fatal: repository '${URL}' not found.`)).toBeVisible();
    await expect(within(dialog).queryByText("fleet.clone_refused")).toBeNull();
    await expect(within(dialog).getByRole("button", { name: /^Clone repository/ })).toBeEnabled();
    await expect(canvas.getByRole("button", { name: DEFAULT_LABEL })).toBeInTheDocument();
  },
};

/** A destination already full: Fleet's words, and where to go from them. */
export const ANonEmptyDestination: Story = {
  name: "A non-empty destination",
  args: { clone: "occupied" },
  play: async ({ canvasElement }) => {
    const { dialog } = await cloneFrom(canvasElement);
    await expect(await within(dialog).findByText("Not cloned")).toBeVisible();
    await expect(within(dialog).getByText("/Users/user/code/storefront already exists and is not empty.")).toBeVisible();
    await expect(within(dialog).getByText("Choose another folder to clone into.")).toBeVisible();
    // Fleet's sentence names the destination, so the preview does not say it a second time.
    await expect(within(dialog).queryByRole("group", { name: "Project location" })).toBeNull();
    await expect(within(dialog).queryByText("fleet.destination_occupied")).toBeNull();
  },
};

/** Escape closes it with nothing sent, and it opens again empty. */
export const ClosedWithNothingDone: Story = {
  name: "Closed with nothing done",
  play: async ({ args, canvasElement }) => {
    const { canvas, dialog } = await opened(canvasElement);
    await userEvent.type(within(dialog).getByLabelText("Project location"), "/Users/user/elsewhere");
    await userEvent.keyboard("{Escape}");
    await expect(canvas.queryByRole("dialog", { name: "Add a repository" })).toBeNull();
    await expect(args.onAdded).not.toHaveBeenCalled();
    const again = await opened(canvasElement);
    await expect(within(again.dialog).getByLabelText("Project location")).toHaveValue("");
    await userEvent.click(within(again.dialog).getByRole("button", { name: /^Cancel/ }));
    await expect(canvas.queryByRole("dialog", { name: "Add a repository" })).toBeNull();
  },
};

const SET_UP = { ...repository(), root: "/Users/user/code/web-app", manifest: { ...repository().manifest!, id: "storefront" } };
const LOOSE = { root: "/Users/user/scratch", records_root: "/records/scratch" };
const API = { root: "/Users/user/code/api", records_root: "/records/api" };
const OLD_API = { root: "/Users/user/old/api", records_root: "/records/old-api" };
const SET_UP_API = { ...repository(), root: "/Users/user/services/api", manifest: { ...repository().manifest!, id: "api" } };

/**
 * A set-up repository reads as its Manifest id and nothing else. A folder nobody set up reads as its
 * name, and takes its parent only where another not set up shares that name.
 */
export const ThePickersNames: Story = {
  name: "The picker's names",
  args: { repositories: [SET_UP, SET_UP_API, LOOSE, API, OLD_API] },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    // `SET_UP` is `listed[0]`, so it is where the picker opens.
    await userEvent.click(canvas.getByRole("button", { name: "storefront" }));
    await expect(canvas.getByRole("menuitem", { name: "storefront" })).toBeInTheDocument();
    await expect(canvas.getByRole("menuitem", { name: "api" })).toBeInTheDocument();
    await expect(canvas.queryByRole("menuitem", { name: "services/api" })).toBeNull();
    // Scoped to the open menu: the left column's Stats panel reads the same
    // two words for a repository's own setup state, #1088.
    await waitFor(() => expect(within(canvas.getByRole("menu")).getByText("Not set up")).toBeVisible());
    await expect(canvas.getByRole("menuitem", { name: "scratch" })).toBeInTheDocument();
    await expect(canvas.getByRole("menuitem", { name: "code/api" })).toBeInTheDocument();
    await expect(canvas.getByRole("menuitem", { name: "old/api" })).toBeInTheDocument();
    await expect(canvas.queryByRole("menuitem", { name: "web-app" })).toBeNull();
  },
};
