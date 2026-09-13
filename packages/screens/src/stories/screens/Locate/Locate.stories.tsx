import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";

import { repository } from "../../../fixtures/build/base";
import { CHOSEN, LocateFrom } from "./Locate";

/**
 * Locate — Journey 3's *Getting in* — from the rail's **Add a repository**: a folder added, or a
 * clone from a URL, landing on Setup for what Fleet now serves. Nothing native opens: the folder
 * dialog answers `/Users/user/scratch`.
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

async function opened(canvasElement: HTMLElement) {
  const canvas = within(canvasElement);
  await userEvent.click(canvas.getByRole("button", { name: "Add a repository" }));
  return { canvas, dialog: canvas.getByRole("dialog", { name: "Add a repository" }) };
}

async function cloneFrom(canvasElement: HTMLElement) {
  const { canvas, dialog } = await opened(canvasElement);
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
    const picker = canvas.getByRole("combobox", { name: "Project" });
    await expect(picker).toHaveValue(CHOSEN);
    await expect(within(within(picker).getByRole("group", { name: "Not set up" })).getByRole("option", { name: "scratch" })).toBeInTheDocument();
    await expect(await canvas.findByRole("region", { name: "Workspaces" })).toBeVisible();
  },
};

/** A fresh install: Fleet serves nothing, so the dialog opens by itself over a rail saying so, and nothing reads as a fault. */
export const NothingServed: Story = {
  name: "Nothing served",
  args: { repositories: [] },
  play: async ({ args, canvasElement }) => {
    const canvas = within(canvasElement);
    const dialog = await canvas.findByRole("dialog", { name: "Add a repository" });
    const picker = canvas.getByRole("combobox", { name: "Project" });
    await expect(within(picker).getByRole("option", { name: "Nothing set up yet" })).toBeInTheDocument();
    await expect(canvas.getByText("Nothing is set up yet")).toBeInTheDocument();
    await expect(canvas.queryByText(/could not be read/)).toBeNull();
    await userEvent.type(within(dialog).getByLabelText("Project location"), CHOSEN);
    await userEvent.click(within(dialog).getByRole("button", { name: /^Add repository/ }));
    await expect(args.onAdded).toHaveBeenCalledWith(CHOSEN);
    // The first repository added lands on Setup.
    await expect(await canvas.findByRole("region", { name: "Workspaces" })).toBeVisible();
    await expect(canvas.getByRole("combobox", { name: "Project" })).toHaveValue(CHOSEN);
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
    const picker = canvas.getByRole("combobox", { name: "Project" });
    await expect(picker).toHaveValue("/Users/user/armada");
    await expect(canvas.queryByRole("region", { name: "Workspaces" })).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "Open Setup" }));
    await expect(await canvas.findByRole("region", { name: "Workspaces" })).toBeVisible();
    await expect(picker).toHaveValue("/Users/user/code/storefront");
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
    await expect(canvas.getByRole("combobox", { name: "Project" })).toHaveValue("/Users/user/armada");
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
    const picker = within(canvasElement).getByRole("combobox", { name: "Project" });
    const setUp = within(picker).getByRole("option", { name: "storefront" });
    await expect(setUp).toHaveAttribute("title", SET_UP.root);
    await expect(within(picker).getByRole("option", { name: "api" })).toHaveAttribute("title", SET_UP_API.root);
    await expect(within(picker).queryByRole("option", { name: "services/api" })).toBeNull();
    const notSetUp = within(within(picker).getByRole("group", { name: "Not set up" }));
    await expect(notSetUp.getByRole("option", { name: "scratch" })).toHaveAttribute("title", LOOSE.root);
    await expect(notSetUp.getByRole("option", { name: "code/api" })).toHaveAttribute("title", API.root);
    await expect(notSetUp.getByRole("option", { name: "old/api" })).toHaveAttribute("title", OLD_API.root);
    await expect(within(picker).queryByRole("option", { name: "web-app" })).toBeNull();
  },
};
